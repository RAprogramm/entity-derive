// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! `OpenAPI` schema overrides carried from the entity to the generated
//! structs.
//!
//! A column whose Rust type says little to `OpenAPI` — `serde_json::Value`
//! for a JSONB column, a newtype over a primitive — documents as a
//! free-form object. utoipa fixes that with `#[schema(...)]` on the
//! field; this module carries such an attribute from the entity onto
//! every generated struct that derives `utoipa::ToSchema`.
//!
//! ```rust,ignore
//! #[field(create, response)]
//! #[schema(value_type = Option<SizeCm>)]
//! pub size_cm: Option<serde_json::Value>,
//! ```
//!
//! The tokens are forwarded verbatim: what utoipa accepts inside
//! `#[schema(...)]` is what works here, and nothing is validated twice.
//! With the `api` feature off no generated struct derives `ToSchema`, so
//! the attribute is dropped rather than emitted onto a struct that
//! cannot interpret it.

use proc_macro2::TokenStream;
use syn::{Attribute, Meta};

/// Extract the token list of a field's `#[schema(...)]` attribute.
///
/// Returns `None` when the field carries no such attribute, or carries
/// one in a form utoipa does not use (`#[schema]`, `#[schema = ...]`).
pub fn parse_schema_attr(attrs: &[Attribute]) -> Option<TokenStream> {
    attrs.iter().find_map(|attr| match &attr.meta {
        Meta::List(list) if list.path.is_ident("schema") => Some(list.tokens.clone()),
        _ => None
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::{Field, parse_quote};

    use super::parse_schema_attr;

    fn field(tokens: TokenStreamAlias) -> Field {
        Field::parse_named.parse2(tokens).expect("field parses")
    }

    type TokenStreamAlias = proc_macro2::TokenStream;
    use syn::parse::Parser as _;

    #[test]
    fn extracts_the_token_list() {
        let f = field(quote! {
            #[schema(value_type = Option<SizeCm>)]
            pub size_cm: Option<serde_json::Value>
        });

        let tokens = parse_schema_attr(&f.attrs).expect("attribute found");

        assert_eq!(tokens.to_string(), "value_type = Option < SizeCm >");
    }

    #[test]
    fn absent_attribute_yields_none() {
        let f: Field = parse_quote! { pub name: String };

        assert!(parse_schema_attr(&f.attrs).is_none());
    }

    #[test]
    fn path_and_name_value_forms_are_ignored() {
        let bare = field(quote! {
            #[schema]
            pub name: String
        });
        let named = field(quote! {
            #[schema = "text"]
            pub name: String
        });

        assert!(parse_schema_attr(&bare.attrs).is_none());
        assert!(parse_schema_attr(&named.attrs).is_none());
    }
}
