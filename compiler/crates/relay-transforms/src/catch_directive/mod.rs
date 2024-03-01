/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

mod catchable_field;
mod validation_message;

use std::borrow::Cow;
use std::mem;
use std::sync::Arc;
use catchable_field::CatchMetadata;
use catchable_field::CatchableField;
use common::ArgumentName;
use common::Diagnostic;
use common::DiagnosticsResult;
use common::DirectiveName;
use common::Location;
use common::NamedItem;
use common::WithLocation;
use graphql_ir::associated_data_impl;
use graphql_ir::Directive;
use graphql_ir::Field;
use graphql_ir::FragmentDefinition;
use graphql_ir::FragmentDefinitionNameMap;
use graphql_ir::InlineFragment;
use graphql_ir::LinkedField;
use graphql_ir::OperationDefinition;
use graphql_ir::Program;
use graphql_ir::ScalarField;
use graphql_ir::Selection;
use graphql_ir::Transformed;
use graphql_ir::TransformedValue;
use graphql_ir::Transformer;
use intern::string_key::Intern;
use intern::string_key::StringKey;
use intern::string_key::StringKeyMap;
use intern::Lookup;
use lazy_static::lazy_static;

use self::validation_message::ValidationMessage;
use crate::DirectiveFinder;
use crate::FragmentAliasMetadata;

lazy_static! {
    pub static ref CATCH_DIRECTIVE_NAME: DirectiveName = DirectiveName("catch".intern());
    pub static ref TO_ARGUMENT: ArgumentName = ArgumentName("to".intern());
    pub static ref CHILDREN_CAN_BUBBLE_METADATA_KEY: DirectiveName =
        DirectiveName("__childrenCanBubbleNull".intern());
    pub static ref RESULT_TO: StringKey = "RESULT".intern();
    static ref NULL_TO: StringKey = "NULL".intern();
    static ref INLINE_DIRECTIVE_NAME: DirectiveName = DirectiveName("inline".intern());
    static ref REQUIRED_DIRECTIVE_NAME: DirectiveName = DirectiveName("required".intern());
    // allowlist, not blocklist
    // only allow to:null first?
    // separately: RESULT with typegen (flow)
    // 
    static ref ALLOW_LISTED_DIRECTIVES: Vec<DirectiveName> = vec![*CATCH_DIRECTIVE_NAME];
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CatchMetadataDirective {
    pub to: CatchTo,
    pub path: StringKey,
}
associated_data_impl!(CatchMetadataDirective);

pub fn catch_directive(program: &Program) -> DiagnosticsResult<Program> {
    let mut transform: CatchDirective<'_> = CatchDirective::new(program);

    let next_program = transform
        .transform_program(program)
        .replace_or_else(|| program.clone());

    if transform.errors.is_empty() {
        Ok(next_program)
    } else {
        Err(transform.errors)
    }
}

// #[derive(Clone)]
struct MaybeCatchField {
    catch: Option<CatchMetadata>,
    field_name: WithLocation<StringKey>,
}

struct CatchField {
    catch: CatchMetadata,
    field_name: WithLocation<StringKey>,
}

struct CatchDirective<'s> {
    program: &'s Program,
    errors: Vec<Diagnostic>,
    path: Vec<&'s str>,
    within_abstract_inline_fragment: bool,
    disallowed_directives_attached: Vec<DirectiveName>,
    parent_inline_fragment_directive: Option<Location>,
    path_catch_map: StringKeyMap<MaybeCatchField>,
    current_node_catch_children: StringKeyMap<CatchField>,
    catch_children_map: StringKeyMap<StringKeyMap<CatchField>>,
    catch_directive_visitor: CatchDirectiveVisitor<'s>,
}

impl<'program> CatchDirective<'program> {
    fn new(program: &'program Program) -> Self {
        Self {
            program,
            errors: Default::default(),
            path: vec![],
            within_abstract_inline_fragment: false,
            disallowed_directives_attached: Default::default(),
            parent_inline_fragment_directive: None,
            path_catch_map: Default::default(),
            current_node_catch_children: Default::default(),
            catch_children_map: Default::default(),
            catch_directive_visitor: CatchDirectiveVisitor {
                program,
                visited_fragments: Default::default(),
            },
        }
    }

    fn reset_state(&mut self) {
        self.path_catch_map = Default::default();
        self.current_node_catch_children = Default::default();
        self.parent_inline_fragment_directive = None;
        self.catch_children_map = Default::default();
    }

    fn assert_not_within_abstract_inline_fragment(&mut self, directive_location: Location) {
        if self.within_abstract_inline_fragment {
            self.errors.push(Diagnostic::error(
                ValidationMessage::CatchWithinAbstractInlineFragment,
                // TODO(T70172661): Also referece the location of the inline fragment, once they have a location.
                directive_location,
            ))
        }
    }

    fn assert_not_disallowed_directives_attached(&mut self, directive_location: Location) {
        if !self.disallowed_directives_attached.is_empty() {
            for disallowed_directive_name in self.disallowed_directives_attached.iter() {
                self.errors.push(Diagnostic::error(
                    ValidationMessage::CatchWithDisallowedDirective {
                        directive_name: *disallowed_directive_name
                    },
                    // TODO(T70172661): Also referece the location of the inline fragment, once they have a location.
                    directive_location,
                ))
            }
        }
    }

    fn assert_not_within_inline_directive(&mut self, directive_location: Location) {
        if let Some(location) = self.parent_inline_fragment_directive {
            self.errors.push(
                Diagnostic::error(
                    ValidationMessage::CatchWithinInlineDirective,
                    directive_location,
                )
                .annotate("The fragment is annotated as @inline here.", location),
            )
        }
    }

    fn assert_compatible_nullability(&mut self, path: StringKey, current: MaybeCatchField) {
        if let Some(previous) = self.path_catch_map.get(&path) {
            if let Some(previous_metadata) = &previous.catch {
                if let Some(current_metadata) = current.catch {
                    if previous_metadata.to != current_metadata.to {
                        self.errors.push(
                            Diagnostic::error(
                                ValidationMessage::CatchToMismatch {
                                    field_name: current.field_name.item,
                                },
                                previous_metadata.to_location,
                            )
                            .annotate(
                                "should be the same as the `action` declared here",
                                current_metadata.to_location,
                            ),
                        )
                    }
                } else {
                    self.errors.push(
                        Diagnostic::error(
                            ValidationMessage::CatchFieldMismatch {
                                field_name: current.field_name.item,
                            },
                            previous.field_name.location,
                        )
                        .annotate("but not @catch here", current.field_name.location),
                    );
                }
            } else if current.catch.is_some() {
                self.errors.push(
                    Diagnostic::error(
                        ValidationMessage::CatchFieldMismatch {
                            field_name: current.field_name.item,
                        },
                        current.field_name.location,
                    )
                    .annotate("but not @catch here", previous.field_name.location),
                )
            }
        } else {
            self.path_catch_map.insert(path, current);
        }
    }

    fn get_catch_metadata<T: CatchableField>(
        &mut self,
        field: &T,
        path_name: StringKey,
    ) -> Option<CatchMetadata> {
        let maybe_catch = match field.catch_metadata() {
            Err(err) => {
                self.errors.push(err);
                return None;
            }
            Ok(catch) => catch,
        };

        let field_name = field.name_with_location(&self.program.schema);

        if let Some(metadata) = maybe_catch {
            self.assert_not_disallowed_directives_attached(metadata.directive_location);
            self.assert_not_within_abstract_inline_fragment(metadata.directive_location);
            self.assert_not_within_inline_directive(metadata.directive_location);
            self.current_node_catch_children.insert(
                path_name,
                CatchField {
                    field_name,
                    catch: metadata,
                },
            );
        }

        self.assert_compatible_nullability(
            path_name,
            MaybeCatchField {
                catch: maybe_catch,
                field_name,
            },
        );
        maybe_catch
    }

    fn assert_compatible_catch_children_severity(&mut self, catch_metadata: CatchMetadata) {
        let parent_action = catch_metadata.to;
        for catch_child in self.current_node_catch_children.values() {
            if catch_child.catch.to < parent_action {
                self.errors.push(
                    Diagnostic::error(
                        ValidationMessage::CatchFieldInvalidNesting {
                            suggested_action: catch_child.catch.to.into(),
                        },
                        catch_metadata.to_location,
                    )
                    .annotate(
                        "so that it can match its parent",
                        catch_child.catch.to_location,
                    ),
                );
            }
        }
    }
    fn assert_compatible_catch_children<T: CatchableField>(
        &mut self,
        field: &T,
        field_path: StringKey,
    ) {
        let previous_catch_children = match self.catch_children_map.get(&field_path) {
            Some(it) => it,
            _ => {
                // We haven't seen any other instances of this field, so there's no validation to perform.
                return;
            }
        };

        // Check if this field has a catch child field which was omitted in a previously encountered parent.
        for (path, catch_child) in self.current_node_catch_children.iter() {
            if !previous_catch_children.contains_key(path) {
                if let Some(other_parent) = self.path_catch_map.get(&field_path) {
                    self.errors.push(
                        Diagnostic::error(
                            ValidationMessage::CatchFieldMissing {
                                field_name: catch_child.field_name.item,
                            },
                            catch_child.field_name.location,
                        )
                        .annotate("but is missing from", other_parent.field_name.location),
                    )
                } else {
                    // We want to give a location of the other parent which is
                    // missing this field. We expect that we will be able to
                    // find it in `self.path_catch_map` since it should
                    // contain data about every visited field in this program
                    // and the other parent _must_ have already been visited.
                    panic!("Could not find other parent node at path \"{}\".", {
                        field_path
                    });
                }
            }
        }

        // Check if a previous reference to this field had a catch child field which we are missing.
        for (path, catch_child) in previous_catch_children.iter() {
            if !self.current_node_catch_children.contains_key(path) {
                self.errors.push(
                    Diagnostic::error(
                        ValidationMessage::CatchFieldMissing {
                            field_name: catch_child.field_name.item,
                        },
                        catch_child.field_name.location,
                    )
                    .annotate(
                        "but is missing from",
                        field.name_with_location(&self.program.schema).location,
                    ),
                )
            }
        }
    }
}

impl<'s> Transformer for CatchDirective<'s> {
    const NAME: &'static str = "CatchDirectiveTransform";
    const VISIT_ARGUMENTS: bool = false;
    const VISIT_DIRECTIVES: bool = false;

    fn transform_fragment(
        &mut self,
        fragment: &FragmentDefinition,
    ) -> Transformed<FragmentDefinition> {
        if !self.catch_directive_visitor.visit_fragment(fragment) {
            return Transformed::Keep;
        }
        self.reset_state();
        self.parent_inline_fragment_directive = fragment
            .directives
            .named(*INLINE_DIRECTIVE_NAME)
            .map(|inline_directive| inline_directive.name.location);

        let selections = self.transform_selections(&fragment.selections);
        let directives = self.transform_directives(&fragment.directives);
        
        // maybe_add_children_can_bubble_metadata_directive(
        //     &fragment.directives,
        //     &self.current_node_catch_children,
        // );
        if selections.should_keep() && directives.should_keep() {
            return Transformed::Keep;
        }
        Transformed::Replace(FragmentDefinition {
            directives: directives.replace_or_else(|| fragment.directives.clone()),
            selections: selections.replace_or_else(|| fragment.selections.clone()),
            ..fragment.clone()
        })
    }

    fn transform_operation(
        &mut self,
        operation: &OperationDefinition,
    ) -> Transformed<OperationDefinition> {
        if !self
            .catch_directive_visitor
            .find(operation.selections.iter().collect())
        {
            return Transformed::Keep;
        }
        self.reset_state();
        let selections = self.transform_selections(&operation.selections);
        let directives = maybe_add_children_can_bubble_metadata_directive(
            &operation.directives,
            &self.current_node_catch_children,
        );
        if selections.should_keep() && directives.should_keep() {
            return Transformed::Keep;
        }
        Transformed::Replace(OperationDefinition {
            directives: directives.replace_or_else(|| operation.directives.clone()),
            selections: selections.replace_or_else(|| operation.selections.clone()),
            ..operation.clone()
        })
    }

    fn transform_scalar_field(&mut self, field: &ScalarField) -> Transformed<Selection> {
        let name = field.alias_or_name(&self.program.schema).lookup();
        self.path.push(name);
        let path_name = self.path.join(".").intern();
        self.path.pop();
        
        self.disallowed_directives_attached = maybe_add_disallowed_directives(&field.directives, &self.disallowed_directives_attached);

        match self.get_catch_metadata(field, path_name) {
            None => Transformed::Keep,
            Some(catch_metadata) => {
                Transformed::Replace(Selection::ScalarField(Arc::new(ScalarField {
                    directives: add_metadata_directive(
                        &field.directives,
                        path_name,
                        catch_metadata.to,
                    ),
                    ..field.clone()
                })))
            }
        }
    }

    fn transform_linked_field(&mut self, field: &LinkedField) -> Transformed<Selection> {
        let name = field.alias_or_name(&self.program.schema).lookup();
        self.path.push(name);
        let path_name = self.path.join(".").intern();

        let maybe_catch_metadata = self.get_catch_metadata(field, path_name);
        let next_directives = match maybe_catch_metadata {
            Some(catch_metadata) => Cow::from(add_metadata_directive(
                &field.directives,
                path_name,
                catch_metadata.to,
            )),
            None => Cow::from(&field.directives),
        };

        

        // Once we've handled our own directive, take the parent's catch
        // children map, leaving behind an empty/default map which our children
        // can populate.
        let parent_node_catch_children = mem::take(&mut self.current_node_catch_children);

        let previous_abstract_fragment =
            mem::replace(&mut self.within_abstract_inline_fragment, false);

        let selections = self.transform_selections(&field.selections);

        self.assert_compatible_catch_children(field, path_name);
        if let Some(catch_metadata) = maybe_catch_metadata {
            self.assert_compatible_catch_children_severity(catch_metadata);
        }

        self.disallowed_directives_attached = maybe_add_disallowed_directives(&field.directives, &self.disallowed_directives_attached);

        let next_directives_with_metadata = maybe_add_children_can_bubble_metadata_directive(
            &next_directives,
            &self.current_node_catch_children,
        );

        self.within_abstract_inline_fragment = previous_abstract_fragment;

        let catch_children = mem::replace(
            &mut self.current_node_catch_children,
            parent_node_catch_children,
        );

        self.catch_children_map.insert(path_name, catch_children);

        self.path.pop();

        if selections.should_keep()
            && next_directives_with_metadata.should_keep()
            && maybe_catch_metadata.is_none()
        {
            Transformed::Keep
        } else {
            Transformed::Replace(Selection::LinkedField(Arc::new(LinkedField {
                directives: next_directives_with_metadata
                    .replace_or_else(|| next_directives.into()),
                selections: selections.replace_or_else(|| field.selections.clone()),
                ..field.clone()
            })))
        }
    }

    fn transform_inline_fragment(&mut self, fragment: &InlineFragment) -> Transformed<Selection> {
        let previous = self.within_abstract_inline_fragment;

        let maybe_alias =
            FragmentAliasMetadata::find(&fragment.directives).map(|metadata| metadata.alias.item);

        if let Some(alias) = maybe_alias {
            self.path.push(alias.lookup())
        } else if let Some(type_) = fragment.type_condition {
            if type_.is_abstract_type() {
                self.within_abstract_inline_fragment = true;
            }
        }

        let next_fragment = self.default_transform_inline_fragment(fragment);

        if maybe_alias.is_some() {
            self.path.pop();
        }

        self.within_abstract_inline_fragment = previous;
        next_fragment
    }


}

fn maybe_add_disallowed_directives(directives: &Vec<Directive>, disallowed_directives_attached: &Vec<DirectiveName>) -> Vec<DirectiveName> {
    let mut next_disallowed_directives: Vec<DirectiveName> = disallowed_directives_attached.clone();

    for directive in directives.iter() {
        if !ALLOW_LISTED_DIRECTIVES.contains(&directive.name.item){
            next_disallowed_directives.push(directive.name.item)
        }
    }
    
    return next_disallowed_directives;
}

fn add_metadata_directive(
    directives: &[Directive],
    path_name: StringKey,
    to: CatchTo,
) -> Vec<Directive> {
    let mut next_directives: Vec<Directive> = Vec::with_capacity(directives.len() + 1);
    next_directives.extend(directives.iter().cloned());
    next_directives.push(
        CatchMetadataDirective {
            to,
            path: path_name,
        }
        .into(),
    );
    next_directives
}

fn maybe_add_children_can_bubble_metadata_directive(
    directives: &[Directive],
    current_node_catch_children: &StringKeyMap<CatchField>,
) -> TransformedValue<Vec<Directive>> {
    let children_can_bubble = current_node_catch_children
        .values()
        .any(|child: &CatchField| child.catch.to != CatchTo::Result);

    if !children_can_bubble {
        return TransformedValue::Keep;
    }
    let mut next_directives: Vec<Directive> = Vec::with_capacity(directives.len() + 1);
    for directive in directives.iter() {
        next_directives.push(directive.clone());
    }

    next_directives.push(Directive {
        name: WithLocation::generated(*CHILDREN_CAN_BUBBLE_METADATA_KEY),
        arguments: vec![],
        data: None,
    });
    TransformedValue::Replace(next_directives)
}

// Possible @catch `to` enum values ordered by severity.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
pub enum CatchTo {
    Result,
    Null,
}

impl Into<StringKey> for CatchTo {
    fn into(self) -> StringKey {
        match self {
            CatchTo::Result => *RESULT_TO,
            CatchTo::Null => *NULL_TO,
        }
    }
}

impl From<StringKey> for CatchTo {
    fn from(to: StringKey) -> Self {
        match to {
            _ if to == *RESULT_TO => Self::Result,
            _ if to == *NULL_TO => Self::Null,
            // Actions that don't conform to the GraphQL schema should have been filtered out in IR validation.
            _ => unreachable!(),
        }
    }
}

struct CatchDirectiveVisitor<'s> {
    program: &'s Program,
    visited_fragments: FragmentDefinitionNameMap<bool>,
}

impl<'s> DirectiveFinder for CatchDirectiveVisitor<'s> {
    fn visit_directive(&self, directive: &Directive) -> bool {
        directive.name.item == *CATCH_DIRECTIVE_NAME
    }

    fn visit_fragment_spread(&mut self, fragment_spread: &graphql_ir::FragmentSpread) -> bool {
        let fragment = self
            .program
            .fragment(fragment_spread.fragment.item)
            .unwrap();
        self.visit_fragment(fragment)
    }
}

impl<'s> CatchDirectiveVisitor<'s> {
    fn visit_fragment(&mut self, fragment: &FragmentDefinition) -> bool {
        if let Some(val) = self.visited_fragments.get(&fragment.name.item) {
            return *val;
        }
        // Avoid dead loop in self-referencing fragments
        self.visited_fragments.insert(fragment.name.item, false);
        let result = self.find(fragment.selections.iter().collect());
        self.visited_fragments.insert(fragment.name.item, result);
        result
    }
}
