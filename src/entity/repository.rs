// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Repository trait generation.
//!
//! Generates an async repository trait with standard CRUD operations.
//! The trait serves as a database abstraction layer, allowing different
//! backend implementations (PostgreSQL, ClickHouse, MongoDB).
//!
//! # Generated Trait
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! #[async_trait]
//! pub trait UserRepository: Send + Sync {
//!     type Error: std::error::Error + Send + Sync;
//!     type Pool;
//!
//!     fn pool(&self) -> &Self::Pool;
//!     async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error>;
//!     async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
//!     async fn update(&self, id: Uuid, dto: UpdateUserRequest) -> Result<User, Self::Error>;
//!     async fn delete(&self, id: Uuid) -> Result<bool, Self::Error>;
//!     async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, Self::Error>;
//! }
//! ```
//!
//! # Associated Types
//!
//! - `Error` — custom error type (default: `sqlx::Error`)
//! - `Pool` — database pool type for transaction support
//!
//! # Conditional Generation
//!
//! Methods are generated based on entity configuration:
//!
//! | Method | Condition |
//! |--------|-----------|
//! | `create` | Entity has `#[field(create)]` fields |
//! | `update` | Entity has `#[field(update)]` fields |
//! | `find_by_id`, `delete`, `list` | Always generated |
//!
//! # SQL Level Control
//!
//! - `sql = "full"` — generates trait + implementation
//! - `sql = "trait"` — generates trait only (implement manually)
//! - `sql = "none"` — no repository generation

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EntityDef, SqlLevel};

/// Generates the repository trait definition.
///
/// Returns an empty `TokenStream` if `sql = "none"` is specified.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let trait_name = format_ident!("{}Repository", entity_name);
    let create_dto = entity.ident_with("Create", "Request");
    let update_dto = entity.ident_with("Update", "Request");

    let id_type = entity
        .id_field()
        .map(|f| f.ty())
        .unwrap_or_else(|| panic!("Entity must have an #[id] field"));

    let create_method = if entity.create_fields().is_empty() {
        TokenStream::new()
    } else {
        quote! { async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error>; }
    };

    let update_method = if entity.update_fields().is_empty() {
        TokenStream::new()
    } else {
        quote! { async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error>; }
    };

    quote! {
        #[async_trait::async_trait]
        #vis trait #trait_name: Send + Sync {
            /// Error type for repository operations.
            type Error: std::error::Error + Send + Sync;

            /// Underlying database pool type.
            type Pool;

            /// Get reference to the underlying database pool.
            ///
            /// Enables transactions and custom queries:
            /// ```ignore
            /// let pool = repo.pool();
            /// let mut tx = pool.begin().await?;
            /// // ... custom operations
            /// tx.commit().await?;
            /// ```
            fn pool(&self) -> &Self::Pool;

            #create_method

            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error>;

            #update_method

            async fn delete(&self, id: #id_type) -> Result<bool, Self::Error>;

            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error>;
        }
    }
}
