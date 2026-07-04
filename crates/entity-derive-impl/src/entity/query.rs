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

/// `#[derive(utoipa::ToSchema)]` when the facade `api` feature is on.
fn api_schema_derive() -> proc_macro2::TokenStream {
    if cfg!(feature = "api") {
        quote::quote! { #[derive(utoipa::ToSchema)] }
    } else {
        proc_macro2::TokenStream::new()
    }
}

use super::parse::{EntityDef, FilterType};
use crate::utils::marker;

/// Generates the query struct for the entity.
///
/// Returns an empty `TokenStream` if no fields have `#[filter]`.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_filters() && !entity.has_sort_fields() {
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
                FilterType::Eq | FilterType::Like | FilterType::Search => {
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
                // Skip: filter_fields() should only return fields with filters
                FilterType::None => vec![]
            }
        })
        .collect();

    let api_derive = api_schema_derive();
    let marker = marker::generated();

    let filter_name = entity.ident_with("", "Filter");

    let (sort_enum, sort_field) = generate_sort_enum(entity);

    quote! {
        #sort_enum

        #marker
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        #api_derive
        #vis struct #query_name {
            #(#field_defs,)*
            #sort_field
            /// Maximum number of results to return.
            pub limit: Option<i64>,
            /// Number of results to skip.
            pub offset: Option<i64>,
        }

        /// Type alias for filter operations (same as Query).
        #vis type #filter_name = #query_name;
    }
}

/// Generate the `{Entity}SortField` enum and the Query `sort` field.
///
/// One `Asc`/`Desc` variant pair per `#[sort]` field. `order_by()`
/// returns a whitelisted static `ORDER BY` fragment, so user input can
/// never inject SQL — unknown values simply fail deserialization.
fn generate_sort_enum(entity: &EntityDef) -> (TokenStream, TokenStream) {
    let sort_fields = entity.sort_fields();
    if sort_fields.is_empty() {
        return (TokenStream::new(), TokenStream::new());
    }

    let vis = &entity.vis;
    let sort_name = entity.ident_with("", "SortField");

    let mut variants = Vec::new();
    let mut arms = Vec::new();
    for field in &sort_fields {
        let column = field.name_str();
        for (suffix, direction) in [("Asc", "ASC"), ("Desc", "DESC")] {
            let variant = format_ident!(
                "{}{}",
                convert_case::Casing::to_case(&column, convert_case::Case::Pascal),
                suffix
            );
            let fragment = format!("{column} {direction}");
            variants.push(quote! { #variant });
            arms.push(quote! { Self::#variant => #fragment });
        }
    }

    let doc = format!("Sortable columns for [`{}`] queries.", entity.name());
    let api_derive = api_schema_derive();
    let marker = marker::generated();

    let sort_enum = quote! {
        #marker
        #[doc = #doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #api_derive
        #vis enum #sort_name {
            #(#variants,)*
        }

        impl #sort_name {
            /// Whitelisted `ORDER BY` fragment for this sort selection.
            #vis fn order_by(&self) -> &'static str {
                match self {
                    #(#arms,)*
                }
            }
        }
    };

    let sort_field = quote! {
        /// Dynamic sort selection (whitelisted columns only).
        pub sort: Option<#sort_name>,
    };

    (sort_enum, sort_field)
}

#[cfg(test)]
mod sort_tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::*;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    fn sortable_entity() -> EntityDef {
        parse_entity(quote! {
            #[entity(table = "posts")]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                #[sort]
                pub title: String,
                #[field(create, response)]
                #[sort]
                #[filter]
                pub views: i64,
            }
        })
    }

    #[test]
    fn sort_enum_generated_with_asc_desc_variants() {
        let code = generate(&sortable_entity()).to_string();
        assert!(code.contains("enum PostSortField"));
        assert!(code.contains("TitleAsc"));
        assert!(code.contains("TitleDesc"));
        assert!(code.contains("ViewsDesc"));
        assert!(code.contains("\"title ASC\""));
        assert!(code.contains("\"views DESC\""));
    }

    #[test]
    fn query_struct_gains_sort_field() {
        let code = generate(&sortable_entity()).to_string();
        assert!(code.contains("pub sort : Option < PostSortField >"));
    }

    #[test]
    fn sort_only_entity_still_generates_query() {
        let entity = parse_entity(quote! {
            #[entity(table = "posts")]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[sort]
                pub title: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("struct PostQuery"));
        assert!(code.contains("PostSortField"));
    }

    #[test]
    fn no_sort_no_enum() {
        let entity = parse_entity(quote! {
            #[entity(table = "posts")]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[filter]
                pub title: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(!code.contains("SortField"));
    }
}
