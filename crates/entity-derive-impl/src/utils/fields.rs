// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Field assignment generation for `From` implementations.
//!
//! Generates the field assignment code used in `From` trait implementations.
//! These utilities handle the boilerplate of mapping fields between types.
//!
//! # Generated Code
//!
//! For a field `name`, different functions generate different assignments:
//!
//! | Function | Generated Code |
//! |----------|----------------|
//! | [`assigns`] | `name: source.name` |
//! | [`assigns_clone`] | `name: source.name.clone()` |
//! | [`create_assigns`] | `name: dto.name` or `name: Uuid::now_v7()` |
//! | [`row_assigns`] | `name: row.name` or with `#[map]` transformations |
//!
//! # Usage
//!
//! These functions are used by `mappers.rs` to generate `From` implementations:
//!
//! ```rust,ignore
//! let assigns = fields::assigns(entity.all_fields(), "row");
//! quote! {
//!     impl From<UserRow> for User {
//!         fn from(row: UserRow) -> Self {
//!             Self { #(#assigns),* }
//!         }
//!     }
//! }
//! ```

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::entity::parse::{FieldDef, MapConfig, UuidVersion};

/// Generates move assignments: `name: source.name`.
///
/// Used when the source is consumed (owned value).
pub fn assigns(fields: &[FieldDef], source: &str) -> Vec<TokenStream> {
    let src = Ident::new(source, Span::call_site());
    fields
        .iter()
        .filter(|f| f.embed.is_none())
        .map(|f: &FieldDef| {
            let name = f.name();
            if let Some((parent, sub)) = &f.embed_origin {
                return quote! { #name: #src.#parent.#sub.clone() };
            }
            quote! { #name: #src.#name }
        })
        .collect()
}

/// Generates clone assignments: `name: source.name.clone()`.
///
/// Used when the source is borrowed and values need to be cloned.
pub fn assigns_clone(fields: &[FieldDef], source: &str) -> Vec<TokenStream> {
    let src = Ident::new(source, Span::call_site());
    fields
        .iter()
        .filter(|f| f.embed.is_none())
        .map(|f: &FieldDef| {
            let name = f.name();
            if let Some((parent, sub)) = &f.embed_origin {
                return quote! { #name: #src.#parent.#sub.clone() };
            }
            quote! { #name: #src.#name.clone() }
        })
        .collect()
}

/// Generates move assignments from field references.
///
/// Same as [`assigns`] but accepts `&[&FieldDef]` instead of `&[FieldDef]`.
pub fn assigns_from_refs(fields: &[&FieldDef], source: &str) -> Vec<TokenStream> {
    let src = Ident::new(source, Span::call_site());
    fields
        .iter()
        .map(|f: &&FieldDef| {
            let name = f.name();
            quote! { #name: #src.#name }
        })
        .collect()
}

/// Generates clone assignments from field references.
///
/// Same as [`assigns_clone`] but accepts `&[&FieldDef]` instead of
/// `&[FieldDef]`.
pub fn assigns_clone_from_refs(fields: &[&FieldDef], source: &str) -> Vec<TokenStream> {
    let src = Ident::new(source, Span::call_site());
    fields
        .iter()
        .map(|f: &&FieldDef| {
            let name = f.name();
            quote! { #name: #src.#name.clone() }
        })
        .collect()
}

/// Generates field assignments for `From<CreateRequest> for Entity`.
///
/// Handles three field categories:
///
/// - **Create fields**: `name: dto.name` (from DTO)
/// - **ID fields**: `id: Uuid::now_v7()` or `Uuid::new_v4()` (auto-generated)
/// - **Other fields**: `name: Default::default()` (auto/skip fields)
pub fn create_assigns(
    all_fields: &[FieldDef],
    create_fields: &[&FieldDef],
    uuid_version: UuidVersion
) -> Vec<TokenStream> {
    all_fields
        .iter()
        .filter(|f| f.embed_origin.is_none())
        .map(|f: &FieldDef| {
            let name = f.name();
            let is_in_create = create_fields.iter().any(|cf: &&FieldDef| cf.name() == name);

            if is_in_create {
                quote! { #name: dto.#name }
            } else if f.is_id() {
                match uuid_version {
                    UuidVersion::V7 => quote! { #name: uuid::Uuid::now_v7() },
                    UuidVersion::V4 => quote! { #name: uuid::Uuid::new_v4() }
                }
            } else {
                quote! { #name: Default::default() }
            }
        })
        .collect()
}

/// Generates field assignments for `From<Row> for Entity`.
///
/// Applies `#[map]` transformations where present. When a field has no
/// mapping configuration, generates a simple field copy.
///
/// # Arguments
///
/// * `fields` — All fields of the entity.
/// * `source` — Source variable name (e.g., `"row"`).
///
/// # Returns
///
/// A vector of `TokenStream` representing field assignments.
///
/// # Examples
///
/// ```rust,ignore
/// let assigns = row_assigns(entity.all_fields(), "row");
/// // For a field with #[map(empty_to_none)]:
/// // Generates: nickname: row.nickname.filter(|s| !s.is_empty())
/// // For a field without #[map]:
/// // Generates: name: row.name
/// ```
pub fn row_assigns(fields: &[FieldDef], source: &str) -> Vec<TokenStream> {
    let src = Ident::new(source, Span::call_site());
    let mut assigns: Vec<TokenStream> = Vec::new();
    for f in fields {
        if f.embed_origin.is_some() {
            continue;
        }
        if let Some(embed) = &f.embed {
            let name = f.name();
            let ty = &f.ty;
            let sub_assigns = embed.subfields.iter().map(|(sub, _)| {
                let column = Ident::new(&format!("{}{}", embed.prefix, sub), Span::call_site());
                quote! { #sub: #src.#column }
            });
            assigns.push(quote! { #name: #ty { #(#sub_assigns),* } });
            continue;
        }
        let name = f.name();
        let map = f.map();
        if matches!(map, MapConfig::None) {
            assigns.push(quote! { #name: #src.#name });
        } else {
            let expr = map.generate(name, &src);
            assigns.push(quote! { #name: #expr });
        }
    }
    assigns
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    fn make_field(tokens: proc_macro2::TokenStream) -> FieldDef {
        let field: syn::Field = parse_quote!(#tokens);
        FieldDef::from_field(&field).unwrap()
    }

    #[test]
    fn row_assigns_simple() {
        let fields = vec![make_field(quote::quote! { pub name: String })];
        let assigns = row_assigns(&fields, "row");
        assert_eq!(assigns.len(), 1);
        let expected = quote! { name: row.name };
        assert_eq!(assigns[0].to_string(), expected.to_string());
    }

    #[test]
    fn row_assigns_multiple() {
        let fields = vec![
            make_field(quote::quote! { pub id: uuid::Uuid }),
            make_field(quote::quote! { pub name: String }),
        ];
        let assigns = row_assigns(&fields, "row");
        assert_eq!(assigns.len(), 2);
    }

    #[test]
    fn row_assigns_empty_to_none() {
        let field = make_field(quote::quote! {
            #[map(empty_to_none)]
            pub nickname: Option<String>
        });
        let assigns = row_assigns(&[field], "row");
        let expected = quote! { nickname: row.nickname.filter(|s| !s.is_empty()) };
        assert_eq!(assigns[0].to_string(), expected.to_string());
    }

    #[test]
    fn row_assigns_unwrap_default() {
        let field = make_field(quote::quote! {
            #[map(unwrap_default)]
            pub age: Option<i32>
        });
        let assigns = row_assigns(&[field], "row");
        let expected = quote! { age: Some(row.age.unwrap_or_default()) };
        assert_eq!(assigns[0].to_string(), expected.to_string());
    }

    #[test]
    fn row_assigns_now() {
        let field = make_field(quote::quote! {
            #[map(now)]
            pub last_seen: Option<chrono::DateTime<chrono::Utc>>
        });
        let assigns = row_assigns(&[field], "row");
        let expected = quote! { last_seen: Some(row.last_seen.unwrap_or_else(chrono::Utc::now)) };
        assert_eq!(assigns[0].to_string(), expected.to_string());
    }

    #[test]
    fn row_assigns_expr() {
        let field = make_field(quote::quote! {
            #[map(expr = "row.raw.parse().unwrap_or(0)")]
            pub score: i32
        });
        let assigns = row_assigns(&[field], "row");
        let expected = quote! { score: row.raw.parse().unwrap_or(0) };
        assert_eq!(assigns[0].to_string(), expected.to_string());
    }

    #[test]
    fn row_assigns_mixed() {
        let fields = vec![
            make_field(quote::quote! { pub id: uuid::Uuid }),
            make_field(quote::quote! {
                #[map(empty_to_none)]
                pub nickname: Option<String>
            }),
            make_field(quote::quote! { pub email: String }),
        ];
        let assigns = row_assigns(&fields, "row");
        assert_eq!(assigns.len(), 3);

        let expected_id = quote! { id: row.id };
        let expected_nickname = quote! { nickname: row.nickname.filter(|s| !s.is_empty()) };
        let expected_email = quote! { email: row.email };

        assert_eq!(assigns[0].to_string(), expected_id.to_string());
        assert_eq!(assigns[1].to_string(), expected_nickname.to_string());
        assert_eq!(assigns[2].to_string(), expected_email.to_string());
    }

    #[test]
    fn row_assigns_source_name() {
        let fields = vec![make_field(quote::quote! { pub name: String })];
        let assigns = row_assigns(&fields, "src");
        let expected = quote! { name: src.name };
        assert_eq!(assigns[0].to_string(), expected.to_string());
    }
}
