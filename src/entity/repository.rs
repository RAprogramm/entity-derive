//! Repository trait generation for the Entity derive macro.
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
            type Error: std::error::Error + Send + Sync;

            #create_method

            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error>;

            #update_method

            async fn delete(&self, id: #id_type) -> Result<bool, Self::Error>;

            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error>;
        }
    }
}
