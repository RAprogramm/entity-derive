//! # revelation-macros
//!
//! Procedural macros for domain code generation.
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
//! Generates: `CreateUserRequest`, `UpdateUserRequest`, `UserResponse`,
//! `UserRepository`, `UserRow`, `InsertableUser`, mappers, SQL impl.

mod entity;
mod utils;

use proc_macro::TokenStream;

/// Derive macro for generating domain boilerplate.
#[proc_macro_derive(Entity, attributes(entity, field, id, auto, validate))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    entity::derive(input)
}
