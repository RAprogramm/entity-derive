// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Default allow-all policy generation.
//!
//! Generates `{Entity}AllowAllPolicy` that permits all operations.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{entity::parse::EntityDef, utils::marker};

/// Generate the allow-all policy implementation.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let trait_name = entity.ident_with("", "Policy");
    let struct_name = entity.ident_with("", "AllowAllPolicy");
    let marker = marker::generated();

    let create_dto = entity.ident_with("Create", "Request");
    let update_dto = entity.ident_with("Update", "Request");
    let id_type = entity.id_field().ty();

    let doc = format!(
        "Default policy for [`{entity_name}`] that allows all operations.\n\n\
         Use this for development or when authorization is handled elsewhere."
    );

    quote! {
        #marker
        #[doc = #doc]
        #[derive(Debug, Clone, Copy, Default)]
        #vis struct #struct_name;

        #[::entity_core::async_trait]
        impl #trait_name for #struct_name {
            type Context = ();
            type Error = std::convert::Infallible;

            async fn can_create(&self, _: &#create_dto, _: &()) -> Result<(), Self::Error> {
                Ok(())
            }

            async fn can_read(&self, _: &#id_type, _: &()) -> Result<(), Self::Error> {
                Ok(())
            }

            async fn can_update(
                &self,
                _: &#id_type,
                _: &#update_dto,
                _: &(),
            ) -> Result<(), Self::Error> {
                Ok(())
            }

            async fn can_delete(&self, _: &#id_type, _: &()) -> Result<(), Self::Error> {
                Ok(())
            }

            async fn can_list(&self, _: &()) -> Result<(), Self::Error> {
                Ok(())
            }
        }
    }
}
