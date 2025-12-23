//! # revelation-macros
//!
//! Procedural macros for the Revelation ecosystem that generate
//! domain boilerplate code from a single entity definition.
//!
//! ## Overview
//!
//! This crate provides the `#[derive(Entity)]` macro that generates:
//!
//! - **DTOs**: `CreateRequest`, `UpdateRequest`, `Response` with validation
//! - **Repository**: Async trait with CRUD operations
//! - **DB Layer**: `Row` (FromRow), `Insertable` structs
//! - **Mappers**: All `From`/`Into` implementations
//! - **SQL**: Optional PostgreSQL query implementations
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use revelation_macros::Entity;
//!
//! #[derive(Entity)]
//! #[entity(table = "users", schema = "core")]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!
//!     #[field(create, update, response)]
//!     pub name: Option<String>,
//!
//!     #[field(create, response)]
//!     pub email: Option<String>,
//!
//!     #[field(skip)]
//!     pub password_hash: String,
//!
//!     #[field(response)]
//!     #[auto]
//!     pub created_at: DateTime<Utc>,
//! }
//! ```
//!
//! This generates:
//! - `CreateUserRequest`, `UpdateUserRequest`, `UserResponse`
//! - `UserRepository` trait
//! - `UserRow`, `InsertableUser`
//! - All necessary `From` implementations
//! - `impl UserRepository for PgPool` (when `sql = "full"`)
//!
//! ## Attributes
//!
//! ### Entity-level (`#[entity(...)]`)
//!
//! | Attribute | Description | Default |
//! |-----------|-------------|---------|
//! | `table` | Database table name | Required |
//! | `schema` | Database schema | `"public"` |
//! | `sql` | SQL generation: `"full"`, `"trait"`, `"none"` | `"full"` |
//!
//! ### Field-level
//!
//! | Attribute | Description |
//! |-----------|-------------|
//! | `#[id]` | Marks primary key field |
//! | `#[field(create)]` | Include in CreateRequest |
//! | `#[field(update)]` | Include in UpdateRequest |
//! | `#[field(response)]` | Include in Response |
//! | `#[field(skip)]` | Exclude from all DTOs |
//! | `#[auto]` | Auto-generated (timestamps) |
//! | `#[validate(...)]` | Pass-through to validator |

mod entity;

use proc_macro::TokenStream;

/// Derive macro for generating domain boilerplate.
///
/// Generates DTOs, Repository trait, DB structs, mappers, and optionally SQL.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Entity)]
/// #[entity(table = "users")]
/// pub struct User {
///     #[id]
///     pub id: Uuid,
///
///     #[field(create, update, response)]
///     pub name: Option<String>,
/// }
/// ```
#[proc_macro_derive(Entity, attributes(entity, field, id, auto, validate))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    entity::derive(input)
}
