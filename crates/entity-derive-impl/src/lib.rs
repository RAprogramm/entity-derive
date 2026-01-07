// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/RAprogramm/entity-derive/main/assets/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/RAprogramm/entity-derive/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    missing_docs,
    rustdoc::missing_crate_level_docs,
    rustdoc::broken_intra_doc_links,
    rust_2018_idioms
)]
#![deny(unsafe_code)]

//! # Quick Navigation
//!
//! - **Getting Started**: See the [crate documentation](crate) above
//! - **Derive Macro**: [`Entity`](macro@Entity) — the main derive macro
//! - **Examples**: Check the [examples directory](https://github.com/RAprogramm/entity-derive/tree/main/examples)
//! - **Wiki**: [Comprehensive guides](https://github.com/RAprogramm/entity-derive/wiki)
//!
//! # Attribute Quick Reference
//!
//! ## Entity-Level `#[entity(...)]`
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(
//!     table = "users",      // Required: database table name
//!     schema = "public",    // Optional: database schema (default: "public")
//!     sql = "full",         // Optional: "full" | "trait" | "none" (default: "full")
//!     dialect = "postgres", // Optional: "postgres" | "clickhouse" | "mongodb" (default: "postgres")
//!     uuid = "v7"           // Optional: "v7" | "v4" (default: "v7")
//! )]
//! pub struct User { /* ... */ }
//! ```
//!
//! ## Field-Level Attributes
//!
//! ```rust,ignore
//! pub struct User {
//!     #[id]                           // Primary key, UUID v7, always in response
//!     pub id: Uuid,
//!
//!     #[field(create, update, response)]  // In all DTOs
//!     pub name: String,
//!
//!     #[field(create, response)]      // Create + Response only
//!     pub email: String,
//!
//!     #[field(skip)]                  // Excluded from all DTOs
//!     pub password_hash: String,
//!
//!     #[field(response)]
//!     #[auto]                         // Auto-generated (excluded from create/update)
//!     pub created_at: DateTime<Utc>,
//!
//!     #[belongs_to(Organization)]     // Foreign key relation
//!     pub org_id: Uuid,
//!
//!     #[filter]                        // Exact match filter in Query struct
//!     pub status: String,
//!
//!     #[filter(like)]                  // ILIKE pattern filter
//!     pub name: String,
//!
//!     #[filter(range)]                 // Range filter (generates from/to fields)
//!     pub created_at: DateTime<Utc>,
//! }
//!
//! // Projections - partial views of the entity
//! #[projection(Public: id, name)]           // UserPublic struct
//! #[projection(Admin: id, name, email)]     // UserAdmin struct
//! ```
//!
//! # Generated Code Overview
//!
//! For a `User` entity, the macro generates:
//!
//! | Generated Type | Description |
//! |----------------|-------------|
//! | `CreateUserRequest` | DTO for `POST` requests |
//! | `UpdateUserRequest` | DTO for `PATCH` requests (all fields `Option<T>`) |
//! | `UserResponse` | DTO for API responses |
//! | `UserRow` | Database row mapping (for `sqlx::FromRow`) |
//! | `InsertableUser` | Struct for `INSERT` statements |
//! | `UserQuery` | Query struct for type-safe filtering (if `#[filter]` used) |
//! | `UserRepository` | Async trait with CRUD methods |
//! | `impl UserRepository for PgPool` | PostgreSQL implementation |
//! | `User{Projection}` | Projection structs (e.g., `UserPublic`, `UserAdmin`) |
//! | `From<...>` impls | Type conversions between all structs |
//!
//! # SQL Generation Modes
//!
//! | Mode | Generates Trait | Generates Impl | Use Case |
//! |------|-----------------|----------------|----------|
//! | `sql = "full"` | ✅ | ✅ | Standard CRUD, simple queries |
//! | `sql = "trait"` | ✅ | ❌ | Custom SQL (joins, CTEs, search) |
//! | `sql = "none"` | ❌ | ❌ | DTOs only, no database layer |
//!
//! # Repository Methods
//!
//! The generated `{Name}Repository` trait includes:
//!
//! ```rust,ignore
//! #[async_trait]
//! pub trait UserRepository: Send + Sync {
//!     type Error: std::error::Error + Send + Sync;
//!
//!     /// Create a new entity
//!     async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error>;
//!
//!     /// Find entity by primary key
//!     async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
//!
//!     /// Update entity with partial data
//!     async fn update(&self, id: Uuid, dto: UpdateUserRequest) -> Result<User, Self::Error>;
//!
//!     /// Delete entity by primary key
//!     async fn delete(&self, id: Uuid) -> Result<bool, Self::Error>;
//!
//!     /// List entities with pagination
//!     async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, Self::Error>;
//!
//!     /// Query entities with type-safe filters (if #[filter] used)
//!     async fn query(&self, query: UserQuery) -> Result<Vec<User>, Self::Error>;
//!
//!     // For each projection, generates optimized SELECT method
//!     async fn find_by_id_public(&self, id: Uuid) -> Result<Option<UserPublic>, Self::Error>;
//!     async fn find_by_id_admin(&self, id: Uuid) -> Result<Option<UserAdmin>, Self::Error>;
//! }
//! ```
//!
//! # Projections
//!
//! Define partial views of entities for optimized SELECT queries:
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users")]
//! #[projection(Public: id, name, avatar)]    // Public profile
//! #[projection(Admin: id, name, email, role)] // Admin view
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!     #[field(create, update, response)]
//!     pub name: String,
//!     #[field(create, response)]
//!     pub email: String,
//!     #[field(update, response)]
//!     pub avatar: Option<String>,
//!     #[field(response)]
//!     pub role: String,
//! }
//!
//! // Generated: UserPublic, UserAdmin structs
//! // Generated: find_by_id_public, find_by_id_admin methods
//!
//! // SQL: SELECT id, name, avatar FROM public.users WHERE id = $1
//! let public = repo.find_by_id_public(user_id).await?;
//! ```
//!
//! # Error Handling
//!
//! The generated implementation uses `sqlx::Error` as the error type.
//! You can wrap it in your application's error type:
//!
//! ```rust,ignore
//! use entity_derive::Entity;
//!
//! #[derive(Entity)]
//! #[entity(table = "users", sql = "trait")]  // Generate trait only
//! pub struct User { /* ... */ }
//!
//! // Implement with your own error type
//! #[async_trait]
//! impl UserRepository for MyDatabase {
//!     type Error = MyAppError;  // Your custom error
//!
//!     async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error> {
//!         // Your implementation
//!     }
//! }
//! ```
//!
//! # Compile-Time Guarantees
//!
//! This crate provides several compile-time guarantees:
//!
//! - **No sensitive data leaks**: Fields marked `#[field(skip)]` are excluded
//!   from all DTOs
//! - **Type-safe updates**: `UpdateRequest` fields are properly wrapped in
//!   `Option`
//! - **Consistent mapping**: `From` impls are always in sync with field
//!   definitions
//! - **SQL injection prevention**: All queries use parameterized bindings
//!
//! # Performance
//!
//! - **Zero runtime overhead**: All code generation happens at compile time
//! - **No reflection**: Generated code is plain Rust structs and impls
//! - **Minimal dependencies**: Only proc-macro essentials (syn, quote, darling)
//!
//! # Comparison with Alternatives
//!
//! | Feature | entity-derive | Diesel | SeaORM |
//! |---------|---------------|--------|--------|
//! | DTO generation | ✅ | ❌ | ❌ |
//! | Repository pattern | ✅ | ❌ | Partial |
//! | Type-safe SQL | ✅ | ✅ | ✅ |
//! | Async support | ✅ | Partial | ✅ |
//! | Boilerplate reduction | ~90% | ~50% | ~60% |

mod entity;
mod error;
mod utils;

use proc_macro::TokenStream;

/// Derive macro for generating complete domain boilerplate from a single entity
/// definition.
///
/// # Overview
///
/// The `Entity` derive macro generates all the boilerplate code needed for a
/// typical CRUD application: DTOs, repository traits, SQL implementations, and
/// type mappers.
///
/// # Generated Types
///
/// For an entity named `User`, the macro generates:
///
/// - **`CreateUserRequest`** — DTO for creation (fields marked with
///   `#[field(create)]`)
/// - **`UpdateUserRequest`** — DTO for updates (fields marked with
///   `#[field(update)]`, wrapped in `Option`)
/// - **`UserResponse`** — DTO for responses (fields marked with
///   `#[field(response)]`)
/// - **`UserRow`** — Database row struct (implements `sqlx::FromRow`)
/// - **`InsertableUser`** — Struct for INSERT operations
/// - **`UserRepository`** — Async trait with CRUD methods
/// - **`impl UserRepository for PgPool`** — PostgreSQL implementation (when
///   `sql = "full"`)
///
/// # Entity Attributes
///
/// Configure the entity using `#[entity(...)]`:
///
/// | Attribute | Required | Default | Description |
/// |-----------|----------|---------|-------------|
/// | `table` | **Yes** | — | Database table name |
/// | `schema` | No | `"public"` | Database schema name |
/// | `sql` | No | `"full"` | SQL generation: `"full"`, `"trait"`, or `"none"` |
/// | `dialect` | No | `"postgres"` | Database dialect: `"postgres"`, `"clickhouse"`, `"mongodb"` |
/// | `uuid` | No | `"v7"` | UUID version for ID: `"v7"` (time-ordered) or `"v4"` (random) |
///
/// # Field Attributes
///
/// | Attribute | Description |
/// |-----------|-------------|
/// | `#[id]` | Primary key. Auto-generates UUID (v7 by default, configurable with `uuid` attribute). Always included in `Response`. |
/// | `#[auto]` | Auto-generated field (e.g., `created_at`). Excluded from `Create`/`Update`. |
/// | `#[field(create)]` | Include in `CreateRequest`. |
/// | `#[field(update)]` | Include in `UpdateRequest`. Wrapped in `Option<T>` if not already. |
/// | `#[field(response)]` | Include in `Response`. |
/// | `#[field(skip)]` | Exclude from ALL DTOs. Use for sensitive data. |
/// | `#[belongs_to(Entity)]` | Foreign key relation. Generates `find_{entity}` method in repository. |
/// | `#[has_many(Entity)]` | One-to-many relation (entity-level). Generates `find_{entities}` method. |
/// | `#[projection(Name: f1, f2)]` | Entity-level. Defines a projection struct with specified fields. |
/// | `#[filter]` | Exact match filter. Generates field in Query struct with `=` comparison. |
/// | `#[filter(like)]` | ILIKE pattern filter. Generates field for text pattern matching. |
/// | `#[filter(range)]` | Range filter. Generates `field_from` and `field_to` fields. |
///
/// Multiple attributes can be combined: `#[field(create, update, response)]`
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,ignore
/// use entity_derive::Entity;
/// use uuid::Uuid;
/// use chrono::{DateTime, Utc};
///
/// #[derive(Entity)]
/// #[entity(table = "users", schema = "core")]
/// pub struct User {
///     #[id]
///     pub id: Uuid,
///
///     #[field(create, update, response)]
///     pub name: String,
///
///     #[field(create, update, response)]
///     pub email: String,
///
///     #[field(skip)]
///     pub password_hash: String,
///
///     #[field(response)]
///     #[auto]
///     pub created_at: DateTime<Utc>,
/// }
/// ```
///
/// ## Custom SQL Implementation
///
/// For complex queries with joins, use `sql = "trait"`:
///
/// ```rust,ignore
/// #[derive(Entity)]
/// #[entity(table = "posts", sql = "trait")]
/// pub struct Post {
///     #[id]
///     pub id: Uuid,
///     #[field(create, update, response)]
///     pub title: String,
///     #[field(create, response)]
///     pub author_id: Uuid,
/// }
///
/// // Implement the repository yourself
/// #[async_trait]
/// impl PostRepository for PgPool {
///     type Error = sqlx::Error;
///
///     async fn find_by_id(&self, id: Uuid) -> Result<Option<Post>, Self::Error> {
///         sqlx::query_as!(Post,
///             r#"SELECT p.*, u.name as author_name
///                FROM posts p
///                JOIN users u ON p.author_id = u.id
///                WHERE p.id = $1"#,
///             id
///         )
///         .fetch_optional(self)
///         .await
///     }
///     // ... other methods
/// }
/// ```
///
/// ## DTOs Only (No Database Layer)
///
/// ```rust,ignore
/// #[derive(Entity)]
/// #[entity(table = "events", sql = "none")]
/// pub struct Event {
///     #[id]
///     pub id: Uuid,
///     #[field(create, response)]
///     pub name: String,
/// }
/// // Only generates CreateEventRequest, EventResponse, etc.
/// // No repository trait or SQL implementation
/// ```
///
/// # Security
///
/// Use `#[field(skip)]` to prevent sensitive data from leaking:
///
/// ```rust,ignore
/// pub struct User {
///     #[field(skip)]
///     pub password_hash: String,  // Never in any DTO
///
///     #[field(skip)]
///     pub api_secret: String,     // Never in any DTO
///
///     #[field(skip)]
///     pub internal_notes: String, // Admin-only, not in public API
/// }
/// ```
///
/// # Generated SQL
///
/// The macro generates parameterized SQL queries that are safe from injection:
///
/// ```sql
/// -- CREATE
/// INSERT INTO schema.table (id, field1, field2, ...)
/// VALUES ($1, $2, $3, ...)
///
/// -- READ
/// SELECT * FROM schema.table WHERE id = $1
///
/// -- UPDATE (dynamic based on provided fields)
/// UPDATE schema.table SET field1 = $1, field2 = $2 WHERE id = $3
///
/// -- DELETE
/// DELETE FROM schema.table WHERE id = $1 RETURNING id
///
/// -- LIST
/// SELECT * FROM schema.table ORDER BY created_at DESC LIMIT $1 OFFSET $2
/// ```
#[proc_macro_derive(
    Entity,
    attributes(
        entity, field, id, auto, validate, belongs_to, has_many, projection, filter, command,
        example
    )
)]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    entity::derive(input)
}

/// Derive macro for generating OpenAPI error response documentation.
///
/// # Overview
///
/// The `EntityError` derive macro generates OpenAPI response documentation
/// from error enum variants, using `#[status(code)]` attributes and doc
/// comments.
///
/// # Example
///
/// ```rust,ignore
/// use entity_derive::EntityError;
/// use thiserror::Error;
/// use utoipa::ToSchema;
///
/// #[derive(Debug, Error, ToSchema, EntityError)]
/// pub enum UserError {
///     /// User with this email already exists
///     #[error("Email already exists")]
///     #[status(409)]
///     EmailExists,
///
///     /// User not found by ID
///     #[error("User not found")]
///     #[status(404)]
///     NotFound,
///
///     /// Invalid credentials provided
///     #[error("Invalid credentials")]
///     #[status(401)]
///     InvalidCredentials,
/// }
/// ```
///
/// # Generated Code
///
/// For `UserError`, generates:
/// - `UserErrorResponses` struct with helper methods
/// - `status_codes()` - returns all error status codes
/// - `descriptions()` - returns all error descriptions
/// - `utoipa_responses()` - returns tuples for OpenAPI responses
///
/// # Attributes
///
/// | Attribute | Required | Description |
/// |-----------|----------|-------------|
/// | `#[status(code)]` | **Yes** | HTTP status code (e.g., 404, 409, 500) |
/// | `/// Doc comment` | No | Used as response description |
#[proc_macro_derive(EntityError, attributes(status))]
pub fn derive_entity_error(input: TokenStream) -> TokenStream {
    error::derive(input)
}
