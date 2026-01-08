// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Helper functions for entity attribute parsing.
//!
//! This module provides utility functions for parsing entity-level attributes
//! that don't fit naturally into darling's derive-based parsing. These helpers
//! handle manual attribute parsing for relations and nested configurations.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    Helper Parsing Functions                         │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Entity Attributes              Helpers                 Output      │
//! │                                                                     │
//! │  #[has_many(Post)]        parse_has_many_attrs()   Vec<Ident>      │
//! │  #[has_many(Comment)]            │                [Post, Comment]   │
//! │         │                        │                                  │
//! │         └────────────────────────┘                                  │
//! │                                                                     │
//! │  #[entity(                 parse_api_attr()        ApiConfig        │
//! │    table = "users",              │                  ├── tag         │
//! │    api(                          │                  ├── security    │
//! │      tag = "Users",              │                  └── handlers    │
//! │      security = "bearer"         │                                  │
//! │    )                             │                                  │
//! │  )]                              │                                  │
//! │         │                        │                                  │
//! │         └────────────────────────┘                                  │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Functions
//!
//! | Function | Input | Output |
//! |----------|-------|--------|
//! | [`parse_has_many_attrs`] | `&[Attribute]` | `Vec<Ident>` |
//! | [`parse_api_attr`] | `&[Attribute]` | `ApiConfig` |
//!
//! # Usage Context
//!
//! These functions are called from [`EntityDef::from_derive_input`] during
//! the entity parsing process. They complement darling's automatic parsing
//! by handling attributes with custom syntax.
//!
//! # Why Not Darling?
//!
//! Some attributes require manual parsing because:
//!
//! | Attribute | Reason |
//! |-----------|--------|
//! | `#[has_many(...)]` | Multiple instances, simple syntax |
//! | `api(...)` | Nested inside `#[entity(...)]`, complex structure |

use syn::{Attribute, Ident};

use super::super::api::{ApiConfig, parse_api_config};
use super::super::field::IndexType;
use super::CompositeIndexDef;

/// Parse `#[has_many(Entity)]` attributes from struct attributes.
///
/// Extracts all has-many relation definitions from the struct's attributes.
/// Each attribute specifies a related entity type for one-to-many
/// relationships.
///
/// # Arguments
///
/// * `attrs` - Slice of syn Attributes from the struct
///
/// # Returns
///
/// Vector of related entity identifiers.
///
/// # Example
///
/// ```rust,ignore
/// // For a User entity with posts and comments:
/// #[has_many(Post)]
/// #[has_many(Comment)]
/// struct User { ... }
///
/// // Returns: vec![Ident("Post"), Ident("Comment")]
/// ```
pub fn parse_has_many_attrs(attrs: &[Attribute]) -> Vec<Ident> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("has_many"))
        .filter_map(|attr| attr.parse_args::<Ident>().ok())
        .collect()
}

/// Parse `api(...)` from `#[entity(...)]` attribute.
///
/// Searches for the `api` key within the entity attribute and parses
/// its nested configuration.
///
/// # Arguments
///
/// * `attrs` - Slice of syn Attributes from the struct
///
/// # Returns
///
/// `ApiConfig` with parsed values, or default if not present.
pub fn parse_api_attr(attrs: &[Attribute]) -> ApiConfig {
    for attr in attrs {
        if !attr.path().is_ident("entity") {
            continue;
        }

        let result: syn::Result<Option<ApiConfig>> =
            attr.parse_args_with(|input: syn::parse::ParseStream<'_>| {
                while !input.is_empty() {
                    let ident: Ident = input.parse()?;

                    if ident == "api" {
                        let content;
                        syn::parenthesized!(content in input);

                        let tokens = content.parse::<proc_macro2::TokenStream>()?;
                        let meta_list = syn::Meta::List(syn::MetaList {
                            path: syn::parse_quote!(api),
                            delimiter: syn::MacroDelimiter::Paren(syn::token::Paren::default()),
                            tokens
                        });

                        if let Ok(config) = parse_api_config(&meta_list) {
                            return Ok(Some(config));
                        }
                    } else if input.peek(syn::Token![=]) {
                        let _: syn::Token![=] = input.parse()?;
                        let _ = input.parse::<syn::Expr>()?;
                    } else if input.peek(syn::token::Paren) {
                        let content;
                        syn::parenthesized!(content in input);
                        let _ = content.parse::<proc_macro2::TokenStream>()?;
                    }

                    if input.peek(syn::Token![,]) {
                        let _: syn::Token![,] = input.parse()?;
                    }
                }
                Ok(None)
            });

        if let Ok(Some(config)) = result {
            return config;
        }
    }

    ApiConfig::default()
}

/// Parse `index(...)` and `unique_index(...)` from `#[entity(...)]` attribute.
///
/// Extracts composite index definitions from the entity attribute.
///
/// # Syntax
///
/// ```text
/// #[entity(
///     table = "users",
///     index(name, email),                    // Btree composite index
///     index(type = "gin", tags),             // GIN index
///     unique_index(tenant_id, email),        // Unique composite
///     index(name = "idx_custom", status),    // Named index
/// )]
/// ```
///
/// # Returns
///
/// Vector of `CompositeIndexDef` with parsed configurations.
pub fn parse_index_attrs(attrs: &[Attribute]) -> Vec<CompositeIndexDef> {
    let mut indexes = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("entity") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("index") {
                if let Ok(idx) = parse_index_content(&meta, false) {
                    indexes.push(idx);
                }
            } else if meta.path.is_ident("unique_index") {
                if let Ok(idx) = parse_index_content(&meta, true) {
                    indexes.push(idx);
                }
            }
            Ok(())
        });
    }

    indexes
}

/// Parse the content of an index(...) or unique_index(...) attribute.
fn parse_index_content(
    meta: &syn::meta::ParseNestedMeta<'_>,
    unique: bool
) -> syn::Result<CompositeIndexDef> {
    let mut columns = Vec::new();
    let mut name = None;
    let mut index_type = IndexType::default();
    let mut where_clause = None;

    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("type") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            index_type = IndexType::from_str(&value.value()).unwrap_or_default();
        } else if nested.path.is_ident("name") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            name = Some(value.value());
        } else if nested.path.is_ident("where") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            where_clause = Some(value.value());
        } else if let Some(ident) = nested.path.get_ident() {
            columns.push(ident.to_string());
        }
        Ok(())
    })?;

    if columns.is_empty() {
        return Err(meta.error("index must have at least one column"));
    }

    Ok(CompositeIndexDef {
        name,
        columns,
        index_type,
        unique,
        where_clause
    })
}
