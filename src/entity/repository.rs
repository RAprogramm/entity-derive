// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
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

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EntityDef, FieldDef, SqlLevel};
use crate::utils::marker;

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

    let relation_methods = generate_relation_methods(entity, id_type);
    let marker = marker::generated();

    quote! {
        #marker
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

            #relation_methods
        }
    }
}

/// Generate relation methods for `#[belongs_to]` and `#[has_many]`.
///
/// For `#[belongs_to(Entity)]`, generates:
/// ```rust,ignore
/// async fn find_{entity_snake}(&self, id: IdType) -> Result<Option<Entity>, Self::Error>;
/// ```
///
/// For `#[has_many(Entity)]`, generates:
/// ```rust,ignore
/// async fn find_{entity_snake_plural}(&self, id: IdType) -> Result<Vec<Entity>, Self::Error>;
/// ```
fn generate_relation_methods(entity: &EntityDef, id_type: &syn::Type) -> TokenStream {
    let belongs_to_methods: Vec<TokenStream> = entity
        .relation_fields()
        .iter()
        .filter_map(|field| generate_belongs_to_method(field, id_type))
        .collect();

    let has_many_methods: Vec<TokenStream> = entity
        .has_many_relations()
        .iter()
        .map(|related| generate_has_many_method(entity, related, id_type))
        .collect();

    quote! {
        #(#belongs_to_methods)*
        #(#has_many_methods)*
    }
}

/// Generate a single `find_{entity}` method for a belongs_to relation.
fn generate_belongs_to_method(field: &FieldDef, id_type: &syn::Type) -> Option<TokenStream> {
    let related_entity = field.belongs_to()?;
    let method_name = format_ident!("find_{}", related_entity.to_string().to_case(Case::Snake));

    Some(quote! {
        /// Find the related entity for this foreign key.
        async fn #method_name(&self, id: #id_type) -> Result<Option<#related_entity>, Self::Error>;
    })
}

/// Generate a `find_{entities}` method for a has_many relation.
fn generate_has_many_method(
    entity: &EntityDef,
    related: &syn::Ident,
    id_type: &syn::Type
) -> TokenStream {
    let related_snake = related.to_string().to_case(Case::Snake);
    let method_name = format_ident!("find_{}s", related_snake);
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let fk_field = format_ident!("{}_id", entity_snake);

    quote! {
        /// Find all related entities for this parent.
        ///
        /// The foreign key field is assumed to be `{parent}_id`.
        async fn #method_name(&self, #fk_field: #id_type) -> Result<Vec<#related>, Self::Error>;
    }
}
