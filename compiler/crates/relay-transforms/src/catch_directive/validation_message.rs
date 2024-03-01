/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use common::DirectiveName;
use intern::string_key::StringKey;
use thiserror::Error;

#[derive(Error, Debug, serde::Serialize)]
#[serde(tag = "type")]
pub(super) enum ValidationMessage {
    #[error(
        "Unexpected @catch within inline fragment on an abstract type. At runtime we cannot know if this field is null, or if it's missing because the inline fragment did not match. Consider using `@alias` to give your inline fragment a name."
    )]
    CatchWithinAbstractInlineFragment,

    #[error("@catch is not supported within @inline fragments.")]
    CatchWithinInlineDirective,


    #[error("@catch is not supported with {directive_name} directive on same field.")]
    CatchWithDisallowedDirective { directive_name: DirectiveName},

    // #[error("Missing `to` argument. @catch expects an `to` argument")]
    // CatchToArgumentCatch,

    #[error(
        "All references to a @catch field must have matching `to` arguments. The `to` used for '{field_name}'"
    )]
    CatchToMismatch { field_name: StringKey },

    #[error(
        "All references to a field must have matching @catch declarations. The field '{field_name}` is @catch here"
    )]
    CatchFieldMismatch { field_name: StringKey },

    #[error(
        "@catch fields must be included in all instances of their parent. The field '{field_name}` is marked as @catch here"
    )]
    CatchFieldMissing { field_name: StringKey },

    #[error(
        "A @catch field may not have an `to` less severe than that of its @catch parent. This @catch directive should probably have `action: {suggested_action}`"
    )]
    CatchFieldInvalidNesting { suggested_action: StringKey },
}
