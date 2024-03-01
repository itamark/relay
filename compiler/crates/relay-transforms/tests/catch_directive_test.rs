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