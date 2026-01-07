// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! EntityDef struct definition.
//!
//! This module defines [`EntityDef`], the central data structure for the entire
//! entity-derive macro system. All code generators receive an `EntityDef` and
//! use its fields to produce the appropriate Rust code.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                       EntityDef Structure                           │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Identity           Configuration          Feature Flags           │
//! │  ├── ident          ├── table              ├── soft_delete         │
//! │  ├── vis            ├── schema             ├── events              │
//! │  └── doc            ├── sql                ├── hooks               │
//! │                     ├── dialect            ├── commands            │
//! │                     ├── uuid               ├── policy              │
//! │                     ├── error              ├── streams             │
//! │                     └── returning          └── transactions        │
//! │                                                                     │
//! │  Fields             Relations              API                      │
//! │  ├── fields[]       ├── has_many[]         └── api_config          │
//! │  └── id_field_index └── projections[]          ├── tag             │
//! │                                                ├── security        │
//! │  Commands                                      └── handlers        │
//! │  └── command_defs[]                                                 │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Field Categories
//!
//! | Category | Accessor | Purpose |
//! |----------|----------|---------|
//! | Identity | `ident`, `vis` | Struct name and visibility |
//! | Table | `table`, `schema` | Database location |
//! | Behavior | `sql`, `dialect`, `uuid` | Code generation options |
//! | Features | `soft_delete`, `events`, etc. | Optional features |
//! | Fields | `fields`, `id_field_index` | Field definitions |
//! | Relations | `has_many`, `projections` | Entity relationships |
//! | Commands | `command_defs` | CQRS command definitions |
//! | API | `api_config` | HTTP handler configuration |
//!
//! # Lifetime
//!
//! `EntityDef` is created once during macro expansion and passed to all
//! generators. It owns all its data (no lifetimes) for simplicity.
//!
//! # Construction
//!
//! Use [`EntityDef::from_derive_input`] (in `constructor.rs`) to create
//! from a `syn::DeriveInput`.

use syn::{Ident, Visibility};

use super::{
    super::{
        api::ApiConfig, command::CommandDef, dialect::DatabaseDialect, field::FieldDef,
        returning::ReturningMode, sql_level::SqlLevel, uuid_version::UuidVersion
    },
    ProjectionDef
};

/// Complete parsed entity definition.
///
/// This is the main data structure passed to all code generators.
/// It contains both entity-level metadata and all field definitions.
///
/// # Construction
///
/// Create via [`EntityDef::from_derive_input`]:
///
/// ```rust,ignore
/// let entity = EntityDef::from_derive_input(&input)?;
/// ```
///
/// # Field Access
///
/// Use the provided methods to access fields by category:
///
/// ```rust,ignore
/// // All fields for Row/Insertable
/// let all = entity.all_fields();
///
/// // Fields for specific DTOs
/// let create_fields = entity.create_fields();
/// let update_fields = entity.update_fields();
/// let response_fields = entity.response_fields();
///
/// // Primary key field (guaranteed to exist)
/// let id = entity.id_field();
/// ```
#[derive(Debug)]
pub struct EntityDef {
    /// Struct identifier (e.g., `User`).
    pub ident: Ident,

    /// Struct visibility.
    ///
    /// Propagated to all generated types so they have the same
    /// visibility as the source entity.
    pub vis: Visibility,

    /// Database table name (e.g., `"users"`).
    pub table: String,

    /// Database schema name (e.g., `"public"`, `"core"`).
    pub schema: String,

    /// SQL generation level controlling what code is generated.
    pub sql: SqlLevel,

    /// Database dialect for code generation.
    pub dialect: DatabaseDialect,

    /// UUID version for ID generation.
    pub uuid: UuidVersion,

    /// Custom error type for repository implementation.
    ///
    /// Defaults to `sqlx::Error`. Custom types must implement
    /// `From<sqlx::Error>` for the `?` operator to work.
    pub error: syn::Path,

    /// All field definitions from the struct.
    pub fields: Vec<FieldDef>,

    /// Index of the primary key field in `fields`.
    ///
    /// Validated at parse time to always be valid.
    pub(super) id_field_index: usize,

    /// Has-many relations defined via `#[has_many(Entity)]`.
    ///
    /// Each entry is the related entity name.
    pub has_many: Vec<Ident>,

    /// Projections defined via `#[projection(Name: field1, field2)]`.
    ///
    /// Each projection defines a subset of fields for a specific view.
    pub projections: Vec<ProjectionDef>,

    /// Whether soft delete is enabled.
    ///
    /// When `true`, the `delete` method sets `deleted_at` instead of removing
    /// the row, and all queries filter out records where `deleted_at IS NOT
    /// NULL`.
    pub soft_delete: bool,

    /// RETURNING clause mode for INSERT/UPDATE operations.
    ///
    /// Controls what data is fetched back from the database after writes.
    pub returning: ReturningMode,

    /// Whether to generate lifecycle events.
    ///
    /// When `true`, generates a `{Entity}Event` enum with variants for
    /// Created, Updated, Deleted, etc.
    pub events: bool,

    /// Whether to generate lifecycle hooks trait.
    ///
    /// When `true`, generates a `{Entity}Hooks` trait with before/after
    /// methods for CRUD operations.
    pub hooks: bool,

    /// Whether to generate CQRS-style commands.
    ///
    /// When `true`, processes `#[command(...)]` attributes.
    pub commands: bool,

    /// Command definitions parsed from `#[command(...)]` attributes.
    ///
    /// Each entry describes a business command (e.g., Register, UpdateEmail).
    pub command_defs: Vec<CommandDef>,

    /// Whether to generate authorization policy trait.
    ///
    /// When `true`, generates `{Entity}Policy` trait and related types.
    pub policy: bool,

    /// Whether to enable real-time streaming.
    ///
    /// When `true`, generates `{Entity}Subscriber` and NOTIFY calls.
    pub streams: bool,

    /// Whether to generate transaction support.
    ///
    /// When `true`, generates transaction repository adapter and builder
    /// methods.
    pub transactions: bool,

    /// API configuration for HTTP handler generation.
    ///
    /// When enabled via `#[entity(api(...))]`, generates axum handlers
    /// with OpenAPI documentation via utoipa.
    pub api_config: ApiConfig,

    /// Documentation comment from the entity struct.
    ///
    /// Extracted from `///` comments for use in OpenAPI tag descriptions.
    pub doc: Option<String>
}
