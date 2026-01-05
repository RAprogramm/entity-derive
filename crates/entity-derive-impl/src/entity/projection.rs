// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Projection struct generation.
//!
//! This module generates projection structs for partial entity views.
//! Each projection defines a subset of fields for specific use cases:
//!
//! | Projection | Use Case |
//! |------------|----------|
//! | `UserPublic` | Public profile (id, name, avatar) |
//! | `UserAdmin` | Admin view (id, name, email, role) |
//! | `PostSummary` | List view (id, title, created_at) |
//!
//! # Definition
//!
//! Projections are defined at entity level:
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users")]
//! #[projection(Public: id, name, avatar)]
//! #[projection(Admin: id, name, email, role)]
//! pub struct User { ... }
//! ```
//!
//! # Generated Code
//!
//! For each projection, generates:
//! - `{Entity}{Projection}` struct with specified fields
//! - `From<{Entity}> for {Entity}{Projection}` implementation

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::EntityDef;
use crate::utils::marker;

/// Generates all projection structs for the entity.
///
/// Returns a combined `TokenStream` containing projection struct definitions
/// and their From implementations.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let projections: Vec<TokenStream> = entity
        .projections
        .iter()
        .map(|proj| generate_projection(entity, proj))
        .collect();

    quote! { #(#projections)* }
}

/// Generate a single projection struct and its From impl.
fn generate_projection(entity: &EntityDef, proj: &super::parse::ProjectionDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let proj_name = format_ident!("{}{}", entity_name, proj.name);

    let field_defs: Vec<TokenStream> = proj
        .fields
        .iter()
        .filter_map(|field_name| {
            entity
                .fields
                .iter()
                .find(|f| f.name() == field_name)
                .map(|f| {
                    let n = f.name();
                    let t = f.ty();
                    quote! { pub #n: #t }
                })
        })
        .collect();

    if field_defs.is_empty() {
        return TokenStream::new();
    }

    let field_mappings: Vec<TokenStream> = proj
        .fields
        .iter()
        .filter_map(|field_name| {
            entity
                .fields
                .iter()
                .find(|f| f.name() == field_name)
                .map(|f| {
                    let n = f.name();
                    quote! { #n: value.#n.clone() }
                })
        })
        .collect();

    let marker = marker::generated();

    quote! {
        #marker
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
        #vis struct #proj_name {
            #(#field_defs),*
        }

        #marker
        impl From<#entity_name> for #proj_name {
            fn from(value: #entity_name) -> Self {
                Self {
                    #(#field_mappings),*
                }
            }
        }

        #marker
        impl From<&#entity_name> for #proj_name {
            fn from(value: &#entity_name) -> Self {
                Self {
                    #(#field_mappings),*
                }
            }
        }
    }
}
