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
    let projection_methods = generate_projection_methods(entity, id_type);
    let soft_delete_methods = generate_soft_delete_methods(entity, id_type);
    let query_method = generate_query_method(entity);
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

            #query_method

            #relation_methods

            #projection_methods

            #soft_delete_methods
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

/// Generate projection methods for `#[projection(Name: fields)]`.
///
/// For each projection, generates:
/// ```rust,ignore
/// async fn find_by_id_public(&self, id: Uuid) -> Result<Option<UserPublic>, Self::Error>;
/// ```
fn generate_projection_methods(entity: &EntityDef, id_type: &syn::Type) -> TokenStream {
    let entity_name = entity.name();

    let methods: Vec<TokenStream> = entity
        .projections
        .iter()
        .map(|proj| {
            let proj_snake = proj.name.to_string().to_case(Case::Snake);
            let method_name = format_ident!("find_by_id_{}", proj_snake);
            let proj_type = format_ident!("{}{}", entity_name, proj.name);

            quote! {
                /// Find entity by ID as projection (optimized SELECT).
                async fn #method_name(&self, id: #id_type) -> Result<Option<#proj_type>, Self::Error>;
            }
        })
        .collect();

    quote! { #(#methods)* }
}

/// Generate soft delete methods when `#[entity(soft_delete)]` is enabled.
///
/// Generates:
/// - `hard_delete` — actual DELETE from database
/// - `restore` — set `deleted_at = NULL` to undelete
/// - `find_by_id_with_deleted` — find without filtering deleted records
/// - `list_with_deleted` — list without filtering deleted records
fn generate_soft_delete_methods(entity: &EntityDef, id_type: &syn::Type) -> TokenStream {
    if !entity.is_soft_delete() {
        return TokenStream::new();
    }

    let entity_name = entity.name();

    quote! {
        /// Permanently remove entity from database.
        ///
        /// Unlike `delete`, this actually removes the row.
        async fn hard_delete(&self, id: #id_type) -> Result<bool, Self::Error>;

        /// Restore a soft-deleted entity.
        ///
        /// Sets `deleted_at = NULL` to make the entity visible again.
        async fn restore(&self, id: #id_type) -> Result<bool, Self::Error>;

        /// Find entity by ID including soft-deleted records.
        ///
        /// Unlike `find_by_id`, this does not filter out deleted records.
        async fn find_by_id_with_deleted(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error>;

        /// List entities including soft-deleted records.
        ///
        /// Unlike `list`, this does not filter out deleted records.
        async fn list_with_deleted(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error>;
    }
}

/// Generate query method when entity has filter fields.
///
/// Generates:
/// ```rust,ignore
/// async fn query(&self, query: UserQuery) -> Result<Vec<User>, Self::Error>;
/// ```
fn generate_query_method(entity: &EntityDef) -> TokenStream {
    if !entity.has_filters() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let query_type = entity.ident_with("", "Query");

    quote! {
        /// Query entities with type-safe filters.
        ///
        /// Supports filtering by fields marked with `#[filter]`.
        async fn query(&self, query: #query_type) -> Result<Vec<#entity_name>, Self::Error>;
    }
}
