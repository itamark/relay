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
