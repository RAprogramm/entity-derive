// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Helper functions for entity parsing.

use syn::{Attribute, Ident};

use super::super::api::{ApiConfig, parse_api_config};

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
