// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Data Transfer Object (DTO) generation.
//!
//! This module generates three DTO structs for API layer separation:
//!
//! | Struct | Purpose | Fields |
//! |--------|---------|--------|
//! | `Create{Name}Request` | Entity creation | `#[field(create)]` fields |
//! | `Update{Name}Request` | Partial updates | `#[field(update)]` fields (wrapped in `Option`) |
//! | `{Name}Response` | API responses | `#[field(response)]` + `#[id]` fields |
//!
//! # Derive Macros
//!
//! All DTOs automatically derive:
//! - `Debug`, `Clone` — standard traits
//! - `serde::Serialize`, `serde::Deserialize` — JSON serialization
//!
//! # Feature Flags
//!
//! - `api` — adds `utoipa::ToSchema` for OpenAPI documentation
//! - `validate` — adds `validator::Validate` for input validation
//!
//! # Field Selection
//!
//! Fields are included based on attributes:
//!
//! ```rust,ignore
//! #[field(create)]           // → CreateRequest only
//! #[field(update)]           // → UpdateRequest only
//! #[field(response)]         // → Response only
//! #[field(create, response)] // → CreateRequest + Response
//! #[field(skip)]             // → excluded from all DTOs
//! #[id]                      // → always in Response
//! #[auto]                    // → excluded from Create/Update
//! ```

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::EntityDef;
use crate::utils::marker;

/// Generates all DTO structs for the entity.
///
/// Returns a combined `TokenStream` containing `CreateRequest`,
/// `UpdateRequest`, and `Response` struct definitions.
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

    let marker = marker::generated();

    quote! {
        #marker
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

    let marker = marker::generated();

    quote! {
        #marker
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

    let marker = marker::generated();

    quote! {
        #marker
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #vis struct #name { #(#field_defs),* }
    }
}
