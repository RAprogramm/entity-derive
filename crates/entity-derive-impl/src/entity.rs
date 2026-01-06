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

mod commands;
mod dto;
mod events;
mod hooks;
mod insertable;
mod mappers;
pub mod parse;
mod policy;
mod projection;
mod query;
mod repository;
mod row;
mod sql;
mod streams;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use self::parse::EntityDef;

/// Main entry point for the Entity derive macro.
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match EntityDef::from_derive_input(&input) {
        Ok(entity) => generate(entity),
        Err(err) => err.write_errors().into()
    }
}

fn generate(entity: EntityDef) -> TokenStream {
    let dto = dto::generate(&entity);
    let projections = projection::generate(&entity);
    let query_struct = query::generate(&entity);
    let events = events::generate(&entity);
    let hooks = hooks::generate(&entity);
    let commands = commands::generate(&entity);
    let policy = policy::generate(&entity);
    let streams = streams::generate(&entity);
    let repository = repository::generate(&entity);
    let row = row::generate(&entity);
    let insertable = insertable::generate(&entity);
    let mappers = mappers::generate(&entity);
    let sql = sql::generate(&entity);

    let expanded = quote! {
        #dto
        #projections
        #query_struct
        #events
        #hooks
        #commands
        #policy
        #streams
        #repository
        #row
        #insertable
        #mappers
        #sql
    };

    expanded.into()
}
