// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Size budget for the generated token stream.
//!
//! Expansion cost is what every downstream build pays, and the token
//! stream the generators emit is what drives it. A criterion benchmark
//! cannot reach this code — nothing may link a proc-macro crate — so
//! the guard is a budget instead: reference entities are expanded and
//! the resulting token count is compared against a recorded ceiling.
//!
//! A failure means the generated code grew. That is not automatically
//! wrong: a new generator legitimately emits more. Read the diff, decide
//! whether the growth is what you intended, and raise the number in the
//! same commit that causes it — the recorded value is a decision, not a
//! measurement to be blessed away.

use proc_macro2::{TokenStream, TokenTree};
use syn::{DeriveInput, parse_quote};

use crate::entity::expand;

/// Tokens emitted for one entity definition, counted through nesting.
///
/// A delimited group is one tree at the top level; counting only those
/// would report a whole function body as a single token.
fn tokens(input: &DeriveInput) -> usize {
    fn count(stream: TokenStream) -> usize {
        stream
            .into_iter()
            .map(|tree| match tree {
                TokenTree::Group(group) => 1 + count(group.stream()),
                _ => 1
            })
            .sum()
    }

    count(expand(input))
}

/// Assert a generated stream stays within its recorded ceiling.
///
/// The lower bound catches the opposite failure: a generator silently
/// dropping out (a feature gate flipped, an early return added) would
/// otherwise pass a size check quietly.
fn assert_within(label: &str, count: usize, ceiling: usize) {
    let floor = ceiling / 2;
    assert!(
        count >= floor,
        "{label} expanded to {count} tokens, less than half the recorded {ceiling}: a generator \
         probably stopped emitting"
    );
    assert!(
        count <= ceiling,
        "{label} expanded to {count} tokens, above the recorded ceiling of {ceiling}: check what \
         the change added and raise the ceiling deliberately"
    );
}

#[test]
fn minimal_entity_stays_small() {
    let input: DeriveInput = parse_quote! {
        #[entity(table = "widgets")]
        pub struct Widget {
            #[id]
            pub id: uuid::Uuid,

            #[field(create, update, response)]
            pub name: String,
        }
    };

    assert_within("minimal entity", tokens(&input), 2_800);
}

#[test]
fn wide_entity_scales_with_columns() {
    let input: DeriveInput = parse_quote! {
        #[entity(table = "widgets")]
        pub struct Widget {
            #[id]
            pub id: uuid::Uuid,

            #[field(create, update, response)]
            pub c1: String,
            #[field(create, update, response)]
            pub c2: String,
            #[field(create, update, response)]
            pub c3: String,
            #[field(create, update, response)]
            pub c4: String,
            #[field(create, update, response)]
            pub c5: String,
            #[field(create, update, response)]
            pub c6: String,
            #[field(create, update, response)]
            pub c7: String,
            #[field(create, update, response)]
            pub c8: String,
            #[field(create, update, response)]
            pub c9: String,
            #[field(create, update, response)]
            pub c10: String,
        }
    };

    assert_within("ten-column entity", tokens(&input), 4_400);
}

#[test]
fn fully_annotated_entity_stays_within_budget() {
    let input: DeriveInput = parse_quote! {
        #[entity(
            table = "widgets",
            migrations,
            soft_delete,
            transactions,
            aggregate_root,
            events,
            upsert(conflict = "slug")
        )]
        #[projection(Card: id, name)]
        pub struct Widget {
            #[id]
            pub id: uuid::Uuid,

            #[field(create, update, response)]
            #[filter(like)]
            #[sort]
            pub name: String,

            #[field(create, response)]
            #[column(unique)]
            pub slug: String,

            #[owner]
            #[field(create, response)]
            pub owner_id: uuid::Uuid,

            #[version]
            #[field(response)]
            #[auto]
            pub version: i32,

            #[field(response)]
            #[auto]
            pub created_at: chrono::DateTime<chrono::Utc>,

            #[field(skip)]
            pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
        }
    };

    assert_within("fully annotated entity", tokens(&input), 9_200);
}
