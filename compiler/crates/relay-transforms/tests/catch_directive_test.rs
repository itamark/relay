/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @generated SignedSource<<854adab7d4107de9ea7820c842e3868b>>
 */

mod catch_directive;

use catch_directive::transform_fixture;
use fixture_tests::test_fixture;

#[tokio::test]
async fn action_argument_omitted() {
    let input = include_str!("catch_directive/fixtures/catch-to-null-included.graphql");
    let expected = include_str!("catch_directive/fixtures/catch-to-null-included.expected");
    test_fixture(transform_fixture, file!(), "catch-to-null-included.graphql", "catch_directive/fixtures/catch-to-null-included.expected", input, expected).await;
}

#[tokio::test]
async fn catch_no_args_included() {
    let input = include_str!("catch_directive/fixtures/catch-no-args-included.graphql");
    let expected = include_str!("catch_directive/fixtures/catch-no-args-included.expected");
    test_fixture(transform_fixture, file!(), "catch-no-args-included.graphql", "catch_directive/fixtures/catch-no-args-included.expected", input, expected).await;
}

#[tokio::test]
async fn catch_and_required_invalid() {
    let input = include_str!("catch_directive/fixtures/catch-and-required-same-field.invalid.graphql");
    let expected = include_str!("catch_directive/fixtures/catch-and-required-same-field.invalid.expected");
    test_fixture(transform_fixture, file!(), "catch-and-required-same-field.invalid.graphql", "catch_directive/fixtures/catch-and-required-same-field.invalid.expected", input, expected).await;
}

#[tokio::test]
async fn catch_paths() {
    let input = include_str!("catch_directive/fixtures/catch-paths.graphql");
    let expected = include_str!("catch_directive/fixtures/catch-paths.expected");
    test_fixture(transform_fixture, file!(), "catch-paths.graphql", "catch_directive/fixtures/catch-paths.expected", input, expected).await;
}

#[tokio::test]
async fn catch_same_field_different_arg_invalid() {
    let input = include_str!("catch_directive/fixtures/catch-duplicate-on-same-linked-field.invalid.graphql");
    let expected = include_str!("catch_directive/fixtures/catch-duplicate-on-same-linked-field.invalid.expected");
    test_fixture(transform_fixture, file!(), "catch-duplicate-on-same-linked-field.invalid.graphql", "catch_directive/fixtures/catch-duplicate-on-same-linked-field.invalid.expected", input, expected).await;
}

#[tokio::test]
async fn conflicting_catch_across_aliased_inline_fragments() {
    let input = include_str!("catch_directive/fixtures/conflicting-catch-across-aliased-inline-fragments.graphql");
    let expected = include_str!("catch_directive/fixtures/conflicting-catch-across-aliased-inline-fragments.expected");
    test_fixture(transform_fixture, file!(), "conflicting-catch-across-aliased-inline-fragments.graphql", "catch_directive/fixtures/conflicting-catch-across-aliased-inline-fragments.expected", input, expected).await;
}

#[tokio::test]
async fn duplicate_field_catch_no_catch_invalid() {
    let input = include_str!("catch_directive/fixtures/duplicate-field-catch-no-catch.invalid.graphql");
    let expected = include_str!("catch_directive/fixtures/duplicate-field-catch-no-catch.invalid.expected");
    test_fixture(transform_fixture, file!(), "duplicate-field-catch-no-catch.invalid.graphql", "catch_directive/fixtures/duplicate-field-catch-no-catch.invalid.expected", input, expected).await;
}

#[tokio::test]
async fn duplicate_field_different_to() {
    let input = include_str!("catch_directive/fixtures/duplicate-field-different-to.invalid.graphql");
    let expected = include_str!("catch_directive/fixtures/duplicate-field-different-to.invalid.expected");
    test_fixture(transform_fixture, file!(), "duplicate-field-different-to.invalid.graphql", "catch_directive/fixtures/duplicate-field-different-to.invalid.expected", input, expected).await;
}
