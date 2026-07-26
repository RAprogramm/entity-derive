// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Entity derive macro implementation.
//!
//! This module contains all code generation logic for the `#[derive(Entity)]`
//! macro. It orchestrates the parsing of entity definitions and delegates code
//! generation to specialized submodules.
//!
//! # Architecture
//!
//! ```text
//! entity.rs (orchestrator)
//! │
//! ├── parse/         → Attribute parsing (EntityDef, FieldDef, CommandDef)
//! │
//! ├── dto.rs         → CreateRequest, UpdateRequest, Response
//! ├── events.rs      → Lifecycle event enum (Created, Updated, etc.)
//! ├── hooks.rs       → Lifecycle hooks trait (before/after CRUD)
//! ├── commands/      → CQRS command pattern
//! │   ├── struct_gen.rs  → Command payload structs
//! │   ├── enum_gen.rs    → Command enum
//! │   ├── result_gen.rs  → Result enum
//! │   └── handler_gen.rs → Handler trait
//! ├── repository.rs  → Repository trait definition
//! ├── row.rs         → Database row struct (sqlx::FromRow)
//! ├── insertable.rs  → Insertable struct for INSERT operations
//! ├── mappers.rs     → From implementations between types
//! │
//! └── sql/           → Database-specific implementations
//!     ├── postgres.rs   → PostgreSQL (sqlx::PgPool)
//!     ├── clickhouse.rs → ClickHouse (planned)
//!     └── mongodb.rs    → MongoDB (planned)
//! ```
//!
//! # Generated Code
//!
//! For an entity like:
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users")]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!     #[field(create, update, response)]
//!     pub name: String,
//! }
//! ```
//!
//! The macro generates:
//!
//! | Type | Purpose |
//! |------|---------|
//! | `CreateUserRequest` | DTO for entity creation |
//! | `UpdateUserRequest` | DTO for partial updates (all fields optional) |
//! | `UserResponse` | DTO for API responses |
//! | `UserRepository` | Async trait with CRUD operations |
//! | `UserRow` | Database row mapping struct |
//! | `InsertableUser` | Struct for INSERT operations |
//! | `impl From<...>` | Conversions between types |
//! | `impl UserRepository for PgPool` | PostgreSQL implementation |

mod api;
#[cfg(feature = "commands")]
mod commands;
mod dto;
#[cfg(feature = "events")]
mod events;
#[cfg(feature = "hooks")]
mod hooks;
mod insertable;
mod mappers;
mod migrations;
#[cfg(feature = "aggregate_root")]
pub mod new_entity;
pub mod parse;
mod policy;
#[cfg(feature = "projections")]
mod projection;
mod query;
mod repository;
mod row;
mod schema_check;
mod sql;
#[cfg(feature = "streams")]
mod streams;
#[cfg(feature = "transactions")]
mod transaction;
mod view;

// The budget only means something when every generator is compiled in;
// with a generator switched off the stream is legitimately smaller.
#[cfg(all(
    test,
    feature = "events",
    feature = "commands",
    feature = "hooks",
    feature = "transactions",
    feature = "aggregate_root",
    feature = "migrations",
    feature = "projections"
))]
mod budget_tests;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use self::parse::EntityDef;

/// Main entry point for the Entity derive macro.
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand(&input).into()
}

/// Run the whole pipeline on a parsed item and return the tokens it
/// produces.
///
/// Kept separate from [`derive`] so it stays callable outside a
/// procedural-macro context: `proc_macro::TokenStream` only exists
/// while the compiler is expanding, which is why the crate's own tests
/// work with `proc_macro2` here.
pub(crate) fn expand(input: &DeriveInput) -> proc_macro2::TokenStream {
    match EntityDef::from_derive_input(input) {
        Ok(entity) => generate(entity),
        Err(err) => err.write_errors()
    }
}

fn generate(entity: EntityDef) -> proc_macro2::TokenStream {
    let dto = dto::generate(&entity);
    let query_struct = query::generate(&entity);
    let policy = policy::generate(&entity);

    #[cfg(feature = "streams")]
    let streams = streams::generate(&entity);
    #[cfg(not(feature = "streams"))]
    let streams = guard_disabled_attribute(&entity, "streams", entity.has_streams());

    #[cfg(feature = "outbox")]
    let outbox_guard = proc_macro2::TokenStream::new();
    #[cfg(not(feature = "outbox"))]
    let outbox_guard = guard_disabled_attribute(&entity, "outbox", entity.has_outbox());
    let api = api::generate(&entity);
    let repository = repository::generate(&entity);
    let row = row::generate(&entity);
    let insertable = insertable::generate(&entity);
    let mappers = mappers::generate(&entity);
    let sql = sql::generate(&entity);
    let view = view::generate(&entity);
    let schema_check = schema_check::generate(&entity);

    // Opt-out generators. Each entity-attribute group is gated behind a
    // Cargo feature so users can shrink their build by switching them
    // off via `default-features = false`. The macro itself still parses
    // every attribute; only the codegen body is skipped when the
    // feature is off. `guard_disabled_attribute` emits a friendly
    // compile_error if a user enables an attribute whose feature is
    // disabled — much clearer than a missing-method error at the call site.

    #[cfg(feature = "events")]
    let events = events::generate(&entity);
    #[cfg(not(feature = "events"))]
    let events = guard_disabled_attribute(&entity, "events", entity.has_events());

    #[cfg(feature = "hooks")]
    let hooks = hooks::generate(&entity);
    #[cfg(not(feature = "hooks"))]
    let hooks = guard_disabled_attribute(&entity, "hooks", entity.has_hooks());

    #[cfg(feature = "commands")]
    let commands = commands::generate(&entity);
    #[cfg(not(feature = "commands"))]
    let commands = guard_disabled_attribute(&entity, "commands", entity.has_commands());

    #[cfg(feature = "transactions")]
    let transaction = transaction::generate(&entity);
    #[cfg(not(feature = "transactions"))]
    let transaction = guard_disabled_attribute(&entity, "transactions", entity.has_transactions());

    #[cfg(feature = "aggregate_root")]
    let new_entity = new_entity::generate(&entity);
    #[cfg(not(feature = "aggregate_root"))]
    let new_entity =
        guard_disabled_attribute(&entity, "aggregate_root", entity.is_aggregate_root());

    #[cfg(feature = "migrations")]
    let migrations = migrations::generate(&entity);
    #[cfg(not(feature = "migrations"))]
    let migrations = guard_disabled_attribute(&entity, "migrations", entity.migrations);

    #[cfg(feature = "projections")]
    let projections = projection::generate(&entity);
    #[cfg(not(feature = "projections"))]
    let projections =
        guard_disabled_attribute(&entity, "projections", !entity.projections.is_empty());

    let expanded = quote! {
        #dto
        #projections
        #query_struct
        #events
        #hooks
        #commands
        #policy
        #streams
        #outbox_guard
        #transaction
        #api
        #repository
        #row
        #view
        #schema_check
        #insertable
        #mappers
        #new_entity
        #sql
        #migrations
    };

    expanded
}

/// Emit a `compile_error!` if the user opted into an entity-attribute
/// group whose Cargo feature is currently disabled.
///
/// `feature_name` is the public feature flag name (e.g. `"commands"`).
/// `is_requested` is the boolean from the entity attribute parser
/// (e.g. `entity.has_commands()`). If the attribute is not used, this
/// returns an empty `TokenStream` and nothing is emitted.
#[allow(dead_code)]
fn guard_disabled_attribute(
    entity: &EntityDef,
    feature_name: &str,
    is_requested: bool
) -> proc_macro2::TokenStream {
    if !is_requested {
        return proc_macro2::TokenStream::new();
    }
    let entity_name = entity.name();
    let msg = format!(
        "entity `{entity_name}` uses an attribute that requires the `{feature_name}` feature of \
         `entity-derive`, but it is currently disabled. Enable it by adding \
         `features = [\"{feature_name}\"]` to your `entity-derive` dependency, or remove the \
         corresponding `#[entity(...)]` / `#[command(...)]` / `#[projection(...)]` attribute."
    );
    quote! { ::core::compile_error!(#msg); }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn parse_minimal_entity() -> EntityDef {
        let input: syn::DeriveInput = parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: ::uuid::Uuid
            }
        };
        EntityDef::from_derive_input(&input).expect("minimal entity must parse")
    }

    #[test]
    fn guard_returns_empty_when_attribute_not_requested() {
        let entity = parse_minimal_entity();
        let tokens = guard_disabled_attribute(&entity, "commands", false);
        assert!(
            tokens.is_empty(),
            "no compile_error must be emitted when the attribute is absent, got: {tokens}"
        );
    }

    #[test]
    fn guard_emits_compile_error_when_attribute_requested_without_feature() {
        let entity = parse_minimal_entity();
        let tokens = guard_disabled_attribute(&entity, "commands", true).to_string();
        assert!(
            tokens.contains("compile_error"),
            "must emit compile_error! token, got: {tokens}"
        );
        assert!(
            tokens.contains("commands"),
            "diagnostic must name the missing feature, got: {tokens}"
        );
        assert!(
            tokens.contains("features = "),
            "diagnostic must show the user how to enable, got: {tokens}"
        );
    }

    #[test]
    fn guard_includes_entity_name_in_diagnostic() {
        let entity = parse_minimal_entity();
        let tokens = guard_disabled_attribute(&entity, "hooks", true).to_string();
        assert!(
            tokens.contains("User"),
            "diagnostic must name the offending entity, got: {tokens}"
        );
    }

    #[test]
    fn guard_message_references_correct_feature_name() {
        let entity = parse_minimal_entity();
        for feature in [
            "events",
            "commands",
            "hooks",
            "transactions",
            "aggregate_root",
            "migrations",
            "projections"
        ] {
            let tokens = guard_disabled_attribute(&entity, feature, true).to_string();
            assert!(
                tokens.contains(feature),
                "diagnostic for `{feature}` must mention it, got: {tokens}"
            );
        }
    }
}
