// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! DTO generation for the Entity derive macro.
//!
//! Generates CreateRequest, UpdateRequest, and Response structs.

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::EntityDef;

/// Generate all DTOs for the entity.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let create = generate_create_dto(entity);
    let update = generate_update_dto(entity);
    let response = generate_response_dto(entity);

    quote! { #create #update #response }
}

fn generate_create_dto(entity: &EntityDef) -> TokenStream {
    let fields = entity.create_fields();
    if fields.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let name = entity.ident_with("Create", "Request");
    let field_defs = fields.iter().map(|f| {
        let n = f.name();
        let t = f.ty();
        quote! { pub #n: #t }
    });

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #[cfg_attr(feature = "validate", derive(validator::Validate))]
        #vis struct #name { #(#field_defs),* }
    }
}

fn generate_update_dto(entity: &EntityDef) -> TokenStream {
    let fields = entity.update_fields();
    if fields.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let name = entity.ident_with("Update", "Request");
    let field_defs = fields.iter().map(|f| {
        let n = f.name();
        let t = f.ty();
        if f.is_option() {
            quote! { pub #n: #t }
        } else {
            quote! { pub #n: Option<#t> }
        }
    });

    quote! {
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #[cfg_attr(feature = "validate", derive(validator::Validate))]
        #vis struct #name { #(#field_defs),* }
    }
}

fn generate_response_dto(entity: &EntityDef) -> TokenStream {
    let fields = entity.response_fields();
    if fields.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let name = entity.ident_with("", "Response");
    let field_defs = fields.iter().map(|f| {
        let n = f.name();
        let t = f.ty();
        quote! { pub #n: #t }
    });

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #vis struct #name { #(#field_defs),* }
    }
}
