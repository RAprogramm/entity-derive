// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Attribute parsing for the Entity derive macro.
//!
//! This module handles extraction of entity metadata from Rust attributes.
//! It uses [`darling`] for entity-level attributes and manual parsing for
//! field-level attributes (which use marker-style syntax).
//!
//! # Architecture
//!
//! ```text
//! parse.rs (coordinator)
//! ├── entity.rs      - Entity-level parsing (EntityDef)
//! ├── field.rs       - Field-level parsing (FieldDef)
//! │   ├── expose.rs  - DTO exposure config (create, update, response, skip)
//! │   └── storage.rs - DB storage config (id, auto)
//! ├── command.rs     - Command pattern parsing (CommandDef, CommandSource)
//! ├── dialect.rs     - Database dialect (Postgres, ClickHouse, MongoDB)
//! ├── sql_level.rs   - SQL generation level (Full, Trait, None)
//! └── uuid_version.rs - UUID version for IDs (V7, V4)
//! ```
//!
//! # Parsing Strategy
//!
//! Entity-level attributes like `#[entity(table = "users", schema = "core")]`
//! are parsed using darling's `FromDeriveInput` derive macro, which provides:
//!
//! - Automatic validation of required fields
//! - Default values for optional fields
//! - Clear error messages for invalid input
//!
//! Field-level attributes like `#[id]`, `#[auto]`, and `#[field(create,
//! update)]` use manual parsing because they're marker attributes that don't
//! fit darling's key-value model well.
//!
//! # Data Structures
//!
//! ```text
//! EntityDef
//! ├── ident: Ident          (struct name, e.g., "User")
//! ├── vis: Visibility       (pub, pub(crate), etc.)
//! ├── table: String         (database table name)
//! ├── schema: String        (database schema)
//! ├── sql: SqlLevel         (generation level)
//! ├── dialect: DatabaseDialect (Postgres, ClickHouse, MongoDB)
//! ├── uuid: UuidVersion     (V7 or V4)
//! ├── commands: bool        (generate command pattern)
//! ├── command_defs: Vec<CommandDef> (parsed #[command(...)])
//! │   └── CommandDef
//! │       ├── name: Ident           (command name, e.g., "Register")
//! │       ├── source: CommandSource (where fields come from)
//! │       ├── requires_id: bool     (needs entity ID)
//! │       ├── result_type: Option   (custom result type)
//! │       └── kind: CommandKindHint (create/update/delete/custom)
//! └── fields: Vec<FieldDef>
//!     └── FieldDef
//!         ├── ident: Ident          (field name)
//!         ├── ty: Type              (field type)
//!         ├── vis: Visibility       (field visibility)
//!         ├── expose: ExposeConfig  (DTO exposure)
//!         │   ├── create: bool      (in CreateRequest)
//!         │   ├── update: bool      (in UpdateRequest)
//!         │   ├── response: bool    (in Response)
//!         │   └── skip: bool        (excluded from DTOs)
//!         └── storage: StorageConfig (DB storage)
//!             ├── is_id: bool       (#[id] present)
//!             └── is_auto: bool     (#[auto] present)
//! ```
//!
//! # Examples
//!
//! ## Basic Parsing
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users")]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!
//!     #[field(create, response)]
//!     pub name: String,
//! }
//!
//! // Parses to:
//! // EntityDef {
//! //     ident: "User",
//! //     table: "users",
//! //     schema: "public",  // default
//! //     sql: SqlLevel::Full,  // default
//! //     dialect: DatabaseDialect::Postgres, // default
//! //     uuid: UuidVersion::V7, // default
//! //     fields: [
//! //         FieldDef { ident: "id", storage.is_id: true, ... },
//! //         FieldDef { ident: "name", expose.create: true, expose.response: true, ... },
//! //     ]
//! // }
//! ```
//!
//! ## Custom Schema and Dialect
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "products", schema = "inventory", dialect = "postgres", uuid = "v4")]
//! pub struct Product { /* ... */ }
//! ```

mod api;
mod command;
mod dialect;
mod entity;
mod field;
mod returning;
mod sql_level;
mod uuid_version;

// Re-exported for handler generation (#77)
#[allow(unused_imports)]
pub use api::ApiConfig;
pub use command::{CommandDef, CommandKindHint, CommandSource};
pub use dialect::DatabaseDialect;
pub use entity::{EntityDef, ProjectionDef};
pub use field::{FieldDef, FilterType};
pub use returning::ReturningMode;
pub use sql_level::SqlLevel;
pub use uuid_version::UuidVersion;
