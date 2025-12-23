//! Repository trait generation for Entity derive macro.
//!
//! Generates async Repository trait with CRUD operations.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EntityDef, SqlLevel};

/// Generate Repository trait for the entity.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let trait_name = format_ident!("{}Repository", entity_name);

    let create_dto = entity.ident_with("Create", "Request");
    let update_dto = entity.ident_with("Update", "Request");

    let id_field = entity.id_field();
    let id_type = id_field
        .map(|f| f.ty())
        .unwrap_or_else(|| panic!("Entity must have an #[id] field"));

    let has_create = !entity.create_fields().is_empty();
    let has_update = !entity.update_fields().is_empty();

    let create_method = if has_create {
        quote! {
            /// Create a new entity.
            async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error>;
        }
    } else {
        TokenStream::new()
    };

    let update_method = if has_update {
        quote! {
            /// Update an existing entity.
            async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error>;
        }
    } else {
        TokenStream::new()
    };

    quote! {
        /// Repository trait for #entity_name persistence operations.
        #[async_trait::async_trait]
        #vis trait #trait_name: Send + Sync {
            /// Error type for repository operations.
            type Error: std::error::Error + Send + Sync;

            #create_method

            /// Find entity by ID.
            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error>;

            #update_method

            /// Delete entity by ID.
            async fn delete(&self, id: #id_type) -> Result<bool, Self::Error>;

            /// List entities with pagination.
            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error>;
        }
    }
}
