// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Policy trait generation.
//!
//! Generates `{Entity}Policy` trait with authorization methods.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{entity::parse::EntityDef, utils::marker};

/// Generate the policy trait.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let trait_name = entity.ident_with("", "Policy");
    let marker = marker::generated();

    let create_dto = entity.ident_with("Create", "Request");
    let update_dto = entity.ident_with("Update", "Request");
    let id_type = entity.id_field().ty();

    let doc = format!("Authorization policy trait for [`{entity_name}`].");
    let create_doc = format!("Check if create operation is allowed for [`{entity_name}`].");
    let read_doc = format!("Check if read operation is allowed for [`{entity_name}`].");
    let update_doc = format!("Check if update operation is allowed for [`{entity_name}`].");
    let delete_doc = format!("Check if delete operation is allowed for [`{entity_name}`].");
    let list_doc = format!("Check if list operation is allowed for [`{entity_name}`].");

    quote! {
        #marker
        #[doc = #doc]
        #[::entity_core::async_trait]
        #vis trait #trait_name: Send + Sync {
            /// Authorization context type (e.g., user session, request context).
            type Context: Send + Sync;

            /// Error type for authorization failures.
            type Error: std::error::Error + Send + Sync;

            #[doc = #create_doc]
            async fn can_create(
                &self,
                dto: &#create_dto,
                ctx: &Self::Context,
            ) -> Result<(), Self::Error>;

            #[doc = #read_doc]
            async fn can_read(
                &self,
                id: &#id_type,
                ctx: &Self::Context,
            ) -> Result<(), Self::Error>;

            #[doc = #update_doc]
            async fn can_update(
                &self,
                id: &#id_type,
                dto: &#update_dto,
                ctx: &Self::Context,
            ) -> Result<(), Self::Error>;

            #[doc = #delete_doc]
            async fn can_delete(
                &self,
                id: &#id_type,
                ctx: &Self::Context,
            ) -> Result<(), Self::Error>;

            #[doc = #list_doc]
            async fn can_list(&self, ctx: &Self::Context) -> Result<(), Self::Error>;
        }
    }
}
