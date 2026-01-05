// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
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

use crate::entity::parse::{FieldDef, UuidVersion};

/// Generates move assignments: `name: source.name`.
///
/// Used when the source is consumed (owned value).
pub fn assigns(fields: &[FieldDef], source: &str) -> Vec<TokenStream> {
    let src = Ident::new(source, Span::call_site());
    fields
        .iter()
        .map(|f: &FieldDef| {
            let name = f.name();
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
        .map(|f: &FieldDef| {
            let name = f.name();
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
