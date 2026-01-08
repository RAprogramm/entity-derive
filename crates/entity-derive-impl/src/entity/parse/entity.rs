// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Entity-level attribute parsing and definition.
//!
//! This module is the heart of the entity-derive macro system. It parses
//! `#[entity(...)]` attributes and produces `EntityDef`, the central data
//! structure that drives all code generation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    Entity Parsing Pipeline                          │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Input                    Parsing                     Output        │
//! │                                                                     │
//! │  #[entity(              EntityDef::                                 │
//! │    table = "users",      from_derive_input()         EntityDef      │
//! │    soft_delete,                │                         │         │
//! │    events                      │                         │         │
//! │  )]                            ▼                         │         │
//! │  struct User {          ┌─────────────┐                  │         │
//! │    #[id]                │ EntityAttrs │ ◄── darling      │         │
//! │    id: Uuid,            │ (entity-lvl)│                  │         │
//! │    #[field(create)]     └─────────────┘                  │         │
//! │    name: String,               │                         │         │
//! │  }                             │                         ▼         │
//! │                          ┌─────────────┐           ┌───────────┐   │
//! │                          │  FieldDef   │ ◄─────────│ EntityDef │   │
//! │                          │ (per field) │           │ + fields  │   │
//! │                          └─────────────┘           └───────────┘   │
//! │                                                          │         │
//! │                                                          ▼         │
//! │                                                   Code Generation  │
//! │                                                   ├── SQL layer    │
//! │                                                   ├── DTO structs  │
//! │                                                   ├── Repository   │
//! │                                                   └── API handlers │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Structure
//!
//! | File | Purpose |
//! |------|---------|
//! | `def.rs` | `EntityDef` struct definition with all fields |
//! | `constructor.rs` | `from_derive_input()` implementation |
//! | `accessors.rs` | Accessor methods for fields and metadata |
//! | `attrs.rs` | `EntityAttrs` darling parsing struct |
//! | `helpers.rs` | Helper functions for parsing relations and API |
//! | `projection.rs` | Projection definition and parsing |
//! | `tests.rs` | Comprehensive unit tests |
//!
//! # Entity Attributes
//!
//! The `#[entity(...)]` attribute supports extensive configuration:
//!
//! ## Required Attributes
//!
//! | Attribute | Description |
//! |-----------|-------------|
//! | `table` | Database table name (e.g., `"users"`) |
//!
//! ## Optional Attributes
//!
//! | Attribute | Default | Description |
//! |-----------|---------|-------------|
//! | `schema` | `"public"` | Database schema |
//! | `sql` | `Full` | SQL generation level |
//! | `dialect` | `Postgres` | Database dialect |
//! | `uuid` | `V7` | UUID version for IDs |
//! | `error` | `sqlx::Error` | Custom error type |
//! | `returning` | `Full` | RETURNING clause mode |
//!
//! ## Feature Flags
//!
//! | Flag | Effect |
//! |------|--------|
//! | `soft_delete` | Enable soft delete with `deleted_at` field |
//! | `events` | Generate `{Entity}Event` enum |
//! | `hooks` | Generate `{Entity}Hooks` trait |
//! | `commands` | Enable CQRS command pattern |
//! | `policy` | Generate authorization policy trait |
//! | `streams` | Enable real-time LISTEN/NOTIFY streaming |
//! | `transactions` | Generate transaction support |
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use crate::entity::parse::EntityDef;
//!
//! // Parse from derive input
//! let entity = EntityDef::from_derive_input(&input)?;
//!
//! // Access entity metadata
//! let table = entity.full_table_name();  // "public.users"
//! let id = entity.id_field();            // FieldDef for #[id] field
//!
//! // Access field categories for DTO generation
//! let create_fields = entity.create_fields();   // #[field(create)]
//! let update_fields = entity.update_fields();   // #[field(update)]
//! let response_fields = entity.response_fields(); // #[field(response)]
//!
//! // Generate related type names
//! let row_ident = entity.ident_with("", "Row");      // UserRow
//! let repo_ident = entity.ident_with("", "Repository"); // UserRepository
//! ```
//!
//! # Field Categories
//!
//! Fields are categorized based on `#[field(...)]` attributes:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    Field Categories                          │
//! ├──────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  create_fields() ──► CreateUserRequest                       │
//! │  ├─► #[field(create)]                                        │
//! │  ├─► NOT #[id]                                               │
//! │  └─► NOT #[auto]                                             │
//! │                                                              │
//! │  update_fields() ──► UpdateUserRequest                       │
//! │  ├─► #[field(update)]                                        │
//! │  ├─► NOT #[id]                                               │
//! │  └─► NOT #[auto]                                             │
//! │                                                              │
//! │  response_fields() ──► UserResponse                          │
//! │  └─► #[field(response)] OR #[id]                             │
//! │                                                              │
//! │  all_fields() ──► UserRow, InsertableUser                    │
//! │  └─► All fields (database layer)                             │
//! │                                                              │
//! └──────────────────────────────────────────────────────────────┘
//! ```

mod accessors;
mod attrs;
mod constructor;
mod def;
mod helpers;
mod index;
mod projection;

pub use attrs::EntityAttrs;
pub use def::EntityDef;
pub use index::{CompositeIndexDef, parse_index_meta};
pub use projection::{ProjectionDef, parse_projection_attrs};

#[cfg(test)]
mod tests;
