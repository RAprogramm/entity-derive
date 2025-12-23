//! DTO generation for Entity derive macro.
//!
//! Generates CreateRequest, UpdateRequest, and Response structs.

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::EntityDef;

/// Generate all DTOs for the entity.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let create_dto = generate_create_dto(entity);
    let update_dto = generate_update_dto(entity);
    let response_dto = generate_response_dto(entity);

    quote! {
        #create_dto
        #update_dto
        #response_dto
    }
}

/// Generate CreateRequest DTO.
fn generate_create_dto(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let name = entity.ident_with("Create", "Request");
    let fields = entity.create_fields();

    if fields.is_empty() {
        return TokenStream::new();
    }

    let field_defs: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            let ty = f.ty();
            quote! { pub #name: #ty }
        })
        .collect();

    quote! {
        /// Request DTO for creating a new entity.
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #[cfg_attr(feature = "validate", derive(validator::Validate))]
        #vis struct #name {
            #(#field_defs),*
        }
    }
}

/// Generate UpdateRequest DTO.
fn generate_update_dto(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let name = entity.ident_with("Update", "Request");
    let fields = entity.update_fields();

    if fields.is_empty() {
        return TokenStream::new();
    }

    // For update, wrap non-Option fields in Option
    let field_defs: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            let ty = f.ty();
            if f.is_option() {
                quote! { pub #name: #ty }
            } else {
                quote! { pub #name: Option<#ty> }
            }
        })
        .collect();

    quote! {
        /// Request DTO for updating an existing entity.
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #[cfg_attr(feature = "validate", derive(validator::Validate))]
        #vis struct #name {
            #(#field_defs),*
        }
    }
}

/// Generate Response DTO.
fn generate_response_dto(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let name = entity.ident_with("", "Response");
    let fields = entity.response_fields();

    if fields.is_empty() {
        return TokenStream::new();
    }

    let field_defs: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            let ty = f.ty();
            quote! { pub #name: #ty }
        })
        .collect();

    quote! {
        /// Response DTO for API output.
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #vis struct #name {
            #(#field_defs),*
        }
    }
}
