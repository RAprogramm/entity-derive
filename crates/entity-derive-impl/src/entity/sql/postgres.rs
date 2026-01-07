// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! PostgreSQL repository implementation generator.
//!
//! Generates `impl {Name}Repository for sqlx::PgPool` with complete CRUD
//! operations. This is the primary database backend, providing full SQL support
//! via sqlx.
//!
//! # Module Structure
//!
//! ```text
//! postgres/
//! ├── mod.rs         — Main generator and public API
//! ├── context.rs     — Generation context with precomputed values
//! ├── crud.rs        — CREATE, READ, UPDATE, DELETE, LIST methods
//! ├── query.rs       — Type-safe query filtering method
//! ├── relations.rs   — belongs_to and has_many relation methods
//! ├── projections.rs — Optimized projection SELECT methods
//! ├── soft_delete.rs — Soft delete support methods
//! └── helpers.rs     — SQL building helper functions
//! ```
//!
//! # Generated Implementation
//!
//! ```rust,ignore
//! #[cfg(feature = "postgres")]
//! #[async_trait]
//! impl UserRepository for sqlx::PgPool {
//!     type Error = sqlx::Error;
//!     type Pool = sqlx::PgPool;
//!
//!     fn pool(&self) -> &Self::Pool { self }
//!
//!     // CRUD methods
//!     async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error>;
//!     async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
//!     async fn update(&self, id: Uuid, dto: UpdateUserRequest) -> Result<User, Self::Error>;
//!     async fn delete(&self, id: Uuid) -> Result<bool, Self::Error>;
//!     async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, Self::Error>;
//!
//!     // Query method (if #[filter] used)
//!     async fn query(&self, query: UserQuery) -> Result<Vec<User>, Self::Error>;
//!
//!     // Relation methods
//!     async fn find_organization(&self, id: Uuid) -> Result<Option<Organization>, Self::Error>;
//!     async fn find_posts(&self, user_id: Uuid) -> Result<Vec<Post>, Self::Error>;
//!
//!     // Projection methods
//!     async fn find_by_id_public(&self, id: Uuid) -> Result<Option<UserPublic>, Self::Error>;
//!
//!     // Soft delete methods (if soft_delete enabled)
//!     async fn hard_delete(&self, id: Uuid) -> Result<bool, Self::Error>;
//!     async fn restore(&self, id: Uuid) -> Result<bool, Self::Error>;
//! }
//! ```
//!
//! # Feature Flag
//!
//! Generated code is gated behind `#[cfg(feature = "postgres")]`.

mod context;
mod crud;
mod notify;
mod projections;
mod query;
mod relations;
mod soft_delete;

pub mod helpers;

pub use context::Context;
use proc_macro2::TokenStream;
use quote::quote;

use crate::{entity::parse::EntityDef, utils::marker};

/// Generate PostgreSQL repository implementation.
///
/// Creates `impl {Name}Repository for sqlx::PgPool` with all CRUD methods,
/// relation methods, projection methods, query method, and soft delete methods.
///
/// # Generated Methods
///
/// | Category | Methods |
/// |----------|---------|
/// | CRUD | `create`, `find_by_id`, `update`, `delete`, `list` |
/// | Query | `query` (if entity has `#[filter]` fields) |
/// | Relations | `find_{parent}`, `find_{children}` |
/// | Projections | `find_by_id_{projection}` |
/// | Soft Delete | `hard_delete`, `restore`, `*_with_deleted` |
pub fn generate(entity: &EntityDef) -> TokenStream {
    let ctx = Context::new(entity);
    let trait_name = &ctx.trait_name;
    let feature = entity.dialect.feature_flag();
    let error_type = entity.error_type();

    let create_impl = ctx.create_method();
    let find_impl = ctx.find_by_id_method();
    let update_impl = ctx.update_method();
    let delete_impl = ctx.delete_method();
    let list_impl = ctx.list_method();
    let query_impl = ctx.query_method();
    let stream_impl = ctx.stream_filtered_method();
    let relation_impls = ctx.relation_methods();
    let projection_impls = ctx.projection_methods();
    let soft_delete_impls = ctx.soft_delete_methods();
    let marker = marker::generated();

    quote! {
        #marker
        #[cfg(feature = #feature)]
        #[async_trait::async_trait]
        impl #trait_name for sqlx::PgPool {
            type Error = #error_type;
            type Pool = sqlx::PgPool;

            fn pool(&self) -> &Self::Pool {
                self
            }

            #create_impl
            #find_impl
            #update_impl
            #delete_impl
            #list_impl
            #query_impl
            #stream_impl
            #relation_impls
            #projection_impls
            #soft_delete_impls
        }
    }
}
