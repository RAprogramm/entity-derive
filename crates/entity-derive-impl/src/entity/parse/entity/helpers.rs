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

use super::{
    super::{
        api::{ApiConfig, parse_api_config},
        field::IndexType
    },
    CompositeIndexDef
};

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
            let is_index = meta.path.is_ident("index");
            let is_unique_index = meta.path.is_ident("unique_index");

            if is_index || is_unique_index {
                if let Ok(idx) = parse_index_content(&meta, is_unique_index) {
                    indexes.push(idx);
                }
            } else if meta.input.peek(syn::Token![=]) {
                // Consume `key = value` style attributes (e.g., table = "users")
                let _: syn::Token![=] = meta.input.parse()?;
                let _: syn::Expr = meta.input.parse()?;
            } else if meta.input.peek(syn::token::Paren) {
                // Consume `key(...)` style attributes we don't handle
                let content;
                syn::parenthesized!(content in meta.input);
                let _: proc_macro2::TokenStream = content.parse()?;
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
        // Check if this is a key = value option by peeking for `=`
        let has_value = nested.input.peek(syn::Token![=]);

        if has_value && nested.path.is_ident("type") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            index_type = IndexType::from_str(&value.value()).unwrap_or_default();
        } else if has_value && nested.path.is_ident("name") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            name = Some(value.value());
        } else if has_value && nested.path.is_ident("where") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            where_clause = Some(value.value());
        } else if let Some(ident) = nested.path.get_ident() {
            // Treat any other identifier as a column name
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

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    // =========================================================================
    // parse_has_many_attrs tests
    // =========================================================================

    #[test]
    fn has_many_empty() {
        let attrs: Vec<syn::Attribute> = vec![];
        let result = parse_has_many_attrs(&attrs);
        assert!(result.is_empty());
    }

    #[test]
    fn has_many_single() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[has_many(Post)])];
        let result = parse_has_many_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].to_string(), "Post");
    }

    #[test]
    fn has_many_multiple() {
        let attrs: Vec<syn::Attribute> = vec![
            parse_quote!(#[has_many(Post)]),
            parse_quote!(#[has_many(Comment)]),
            parse_quote!(#[has_many(Like)]),
        ];
        let result = parse_has_many_attrs(&attrs);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].to_string(), "Post");
        assert_eq!(result[1].to_string(), "Comment");
        assert_eq!(result[2].to_string(), "Like");
    }

    #[test]
    fn has_many_ignores_other_attrs() {
        let attrs: Vec<syn::Attribute> = vec![
            parse_quote!(#[derive(Debug)]),
            parse_quote!(#[has_many(Post)]),
            parse_quote!(#[entity(table = "users")]),
        ];
        let result = parse_has_many_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].to_string(), "Post");
    }

    // =========================================================================
    // parse_api_attr tests
    // =========================================================================

    #[test]
    fn api_attr_default_when_missing() {
        let attrs: Vec<syn::Attribute> = vec![];
        let result = parse_api_attr(&attrs);
        assert!(result.tag.is_none());
        assert!(result.security.is_none());
    }

    #[test]
    fn api_attr_ignores_non_entity() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[derive(Debug)])];
        let result = parse_api_attr(&attrs);
        assert!(result.tag.is_none());
    }

    #[test]
    fn api_attr_with_tag() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "users", api(tag = "Users API"))])];
        let result = parse_api_attr(&attrs);
        assert_eq!(result.tag, Some("Users API".to_string()));
    }

    #[test]
    fn api_attr_with_security() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "users", api(security = "bearer"))])];
        let result = parse_api_attr(&attrs);
        assert!(result.security.is_some());
    }

    #[test]
    fn api_attr_entity_without_api() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[entity(table = "users")])];
        let result = parse_api_attr(&attrs);
        assert!(result.tag.is_none());
    }

    // =========================================================================
    // parse_index_attrs tests
    // =========================================================================

    #[test]
    fn index_attrs_empty() {
        let attrs: Vec<syn::Attribute> = vec![];
        let result = parse_index_attrs(&attrs);
        assert!(result.is_empty());
    }

    #[test]
    fn index_attrs_no_indexes() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[entity(table = "users")])];
        let result = parse_index_attrs(&attrs);
        assert!(result.is_empty());
    }

    #[test]
    fn index_attrs_single_column() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "users", index(email))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].columns, vec!["email"]);
        assert!(!result[0].unique);
        assert_eq!(result[0].index_type, IndexType::BTree);
    }

    #[test]
    fn index_attrs_multiple_columns() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "users", index(name, email))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].columns, vec!["name", "email"]);
    }

    #[test]
    fn index_attrs_unique() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "users", unique_index(tenant_id, email))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert!(result[0].unique);
        assert_eq!(result[0].columns, vec!["tenant_id", "email"]);
    }

    #[test]
    fn index_attrs_with_type_gin() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "posts", index(type = "gin", tags))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index_type, IndexType::Gin);
        assert_eq!(result[0].columns, vec!["tags"]);
    }

    #[test]
    fn index_attrs_with_type_gist() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "locations", index(type = "gist", coordinates))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index_type, IndexType::Gist);
    }

    #[test]
    fn index_attrs_with_type_brin() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "logs", index(type = "brin", created_at))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index_type, IndexType::Brin);
    }

    #[test]
    fn index_attrs_with_type_hash() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "cache", index(type = "hash", key))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index_type, IndexType::Hash);
    }

    #[test]
    fn index_attrs_with_custom_name() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "users", index(name = "idx_custom", status))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, Some("idx_custom".to_string()));
        assert_eq!(result[0].columns, vec!["status"]);
    }

    #[test]
    fn index_attrs_with_where_clause() {
        let attrs: Vec<syn::Attribute> = vec![
            parse_quote!(#[entity(table = "users", index(email, where = "deleted_at IS NULL"))]),
        ];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].where_clause,
            Some("deleted_at IS NULL".to_string())
        );
    }

    #[test]
    fn index_attrs_multiple_indexes() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[entity(
                table = "users",
                index(email),
                unique_index(tenant_id, email),
                index(type = "gin", tags)
            )]
        )];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 3);

        assert_eq!(result[0].columns, vec!["email"]);
        assert!(!result[0].unique);

        assert_eq!(result[1].columns, vec!["tenant_id", "email"]);
        assert!(result[1].unique);

        assert_eq!(result[2].columns, vec!["tags"]);
        assert_eq!(result[2].index_type, IndexType::Gin);
    }

    #[test]
    fn index_attrs_all_options() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[entity(
                table = "users",
                unique_index(name = "idx_active_users", type = "btree", email, where = "active = true")
            )]
        )];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert!(result[0].unique);
        assert_eq!(result[0].name, Some("idx_active_users".to_string()));
        assert_eq!(result[0].index_type, IndexType::BTree);
        assert_eq!(result[0].columns, vec!["email"]);
        assert_eq!(result[0].where_clause, Some("active = true".to_string()));
    }

    #[test]
    fn index_attrs_ignores_non_entity() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[derive(Debug)])];
        let result = parse_index_attrs(&attrs);
        assert!(result.is_empty());
    }

    #[test]
    fn index_attrs_unknown_type_defaults_to_btree() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[entity(table = "users", index(type = "unknown", col))])];
        let result = parse_index_attrs(&attrs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index_type, IndexType::BTree);
    }
}
