// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Query struct generation.
//!
//! This module generates a query struct for type-safe filtering.
//! Fields marked with `#[filter]` become optional filter parameters.
//!
//! # Generated Code
//!
//! For an entity with filter fields:
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users")]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!
//!     #[field(create, update, response)]
//!     #[filter]
//!     pub name: String,
//!
//!     #[field(response)]
//!     #[auto]
//!     #[filter(range)]
//!     pub created_at: DateTime<Utc>,
//! }
//! ```
//!
//! Generates:
//!
//! ```rust,ignore
//! #[derive(Debug, Clone, Default)]
//! pub struct UserQuery {
//!     pub name: Option<String>,
//!     pub created_at_from: Option<DateTime<Utc>>,
//!     pub created_at_to: Option<DateTime<Utc>>,
//!     pub limit: Option<i64>,
//!     pub offset: Option<i64>,
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EntityDef, FilterType};
use crate::utils::marker;

/// Generates the query struct for the entity.
///
/// Returns an empty `TokenStream` if no fields have `#[filter]`.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_filters() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let query_name = entity.ident_with("", "Query");

    let filter_fields = entity.filter_fields();
    let field_defs: Vec<TokenStream> = filter_fields
        .iter()
        .flat_map(|f| {
            let name = f.name();
            let ty = f.ty();
            let filter = f.filter();

            match filter.filter_type {
                FilterType::Eq | FilterType::Like => {
                    vec![quote! { pub #name: Option<#ty> }]
                }
                FilterType::Range => {
                    let from_name = format_ident!("{}_from", name);
                    let to_name = format_ident!("{}_to", name);
                    vec![
                        quote! { pub #from_name: Option<#ty> },
                        quote! { pub #to_name: Option<#ty> },
                    ]
                }
                FilterType::None => vec![]
            }
        })
        .collect();

    let marker = marker::generated();

    quote! {
        #marker
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #vis struct #query_name {
            #(#field_defs,)*
            /// Maximum number of results to return.
            pub limit: Option<i64>,
            /// Number of results to skip.
            pub offset: Option<i64>,
        }
    }
}
