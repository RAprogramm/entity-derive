// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Expansion benchmarks for the derive macros.
//!
//! Nothing may link a proc-macro crate, so the macro sources are compiled
//! straight into this benchmark binary as plain modules. The pipelines are
//! driven through their `proc_macro2` entry points, which work outside of
//! a compiler expansion context.

// The `derive` entry points reference `proc_macro`, which is only in the
// extern prelude of the proc-macro library target itself.
extern crate proc_macro;

// Declared inside an inline module so the `#[path]` resolves each file and
// its submodules exactly like the library does, then re-exported at the
// crate root where the sources' `crate::` paths expect them.
#[allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
#[rustfmt::skip]
#[path = "../src"]
mod sources {
    pub mod entity;
    pub mod error;
    pub mod utils;
    pub mod value_object;
}

use divan::{Bencher, black_box};
use sources::{entity, error, utils, value_object};
use syn::{DeriveInput, parse_quote};

fn main() {
    divan::main();
}

fn minimal_entity() -> DeriveInput {
    parse_quote! {
        #[entity(table = "widgets")]
        pub struct Widget {
            #[id]
            pub id: uuid::Uuid,

            #[field(create, update, response)]
            pub name: String,
        }
    }
}

fn wide_entity() -> DeriveInput {
    parse_quote! {
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
    }
}

fn fully_annotated_entity() -> DeriveInput {
    parse_quote! {
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
    }
}

fn related_entity() -> DeriveInput {
    parse_quote! {
        #[entity(table = "posts", schema = "blog", migrations)]
        #[has_many(Comment)]
        pub struct Post {
            #[id]
            pub id: uuid::Uuid,

            #[field(create, update, response)]
            #[column(varchar = 200)]
            #[filter(like)]
            pub title: String,

            #[field(create, update, response)]
            pub body: String,

            #[field(create, response)]
            #[belongs_to(User, on_delete = "cascade")]
            #[filter]
            pub author_id: uuid::Uuid,

            #[field(create, update, response)]
            #[column(check = "score >= 0")]
            #[filter(range)]
            pub score: i32,

            #[field(response)]
            #[auto]
            #[filter(range)]
            pub created_at: chrono::DateTime<chrono::Utc>,
        }
    }
}

fn error_enum() -> DeriveInput {
    parse_quote! {
        pub enum UserError {
            /// User with this email already exists
            #[status(409)]
            EmailExists,
            /// User not found by ID
            #[status(404)]
            NotFound,
            /// Invalid credentials provided
            #[status(401)]
            InvalidCredentials,
            /// Account is locked
            #[status(423)]
            Locked,
            /// Unexpected failure
            #[status(500)]
            Internal,
        }
    }
}

fn value_object_enum() -> DeriveInput {
    parse_quote! {
        #[value_object(pg_type = "order_status")]
        pub enum OrderStatus {
            Pending,
            Confirmed,
            Shipped,
            Delivered,
            Cancelled,
            Refunded,
        }
    }
}

const ENTITIES: &[&str] = &["minimal", "wide", "fully_annotated", "related"];

fn entity_input(name: &str) -> DeriveInput {
    match name {
        "minimal" => minimal_entity(),
        "wide" => wide_entity(),
        "fully_annotated" => fully_annotated_entity(),
        "related" => related_entity(),
        _ => unreachable!("unknown entity {name}")
    }
}

/// Full `#[derive(Entity)]` pipeline: attribute parsing plus every generator.
#[divan::bench(args = ENTITIES)]
fn entity_expand(bencher: Bencher<'_, '_>, name: &str) {
    bencher
        .with_inputs(|| entity_input(name))
        .bench_local_refs(|input| entity::expand(black_box(input)));
}

/// Attribute parsing alone (`EntityDef` construction through darling).
#[divan::bench(args = ENTITIES)]
fn entity_parse(bencher: Bencher<'_, '_>, name: &str) {
    bencher
        .with_inputs(|| entity_input(name))
        .bench_local_refs(|input| entity::parse::EntityDef::from_derive_input(black_box(input)));
}

/// Expansion followed by rendering to source text, as the compiler consumes it.
#[divan::bench(args = ENTITIES)]
fn entity_expand_to_string(bencher: Bencher<'_, '_>, name: &str) {
    bencher
        .with_inputs(|| entity_input(name))
        .bench_local_refs(|input| entity::expand(black_box(input)).to_string());
}

/// Parsing the item source with `syn` and expanding it, end to end.
#[divan::bench(args = ENTITIES)]
fn entity_parse_source_and_expand(bencher: Bencher<'_, '_>, name: &str) {
    let item = entity_input(name);
    let source = quote::quote!(#item).to_string();
    bencher.bench_local(|| {
        let input: DeriveInput = syn::parse_str(black_box(&source)).expect("valid item");
        entity::expand(&input)
    });
}

#[divan::bench]
fn entity_error_expand(bencher: Bencher<'_, '_>) {
    bencher
        .with_inputs(error_enum)
        .bench_local_refs(|input| error::generate(black_box(input)));
}

#[divan::bench]
fn value_object_expand(bencher: Bencher<'_, '_>) {
    bencher
        .with_inputs(value_object_enum)
        .bench_local_refs(|input| value_object::generate(black_box(input)));
}
