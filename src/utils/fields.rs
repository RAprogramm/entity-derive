//! Field assignment utilities for `From` implementations.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::entity::parse::FieldDef;

/// Generate `name: source.name` assignments.
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

/// Generate `name: source.name.clone()` assignments.
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

/// Generate `name: source.name` assignments from references.
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

/// Generate `name: source.name.clone()` assignments from references.
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

/// Generate field assignments for `From<CreateRequest>`.
pub fn create_assigns(all_fields: &[FieldDef], create_fields: &[&FieldDef]) -> Vec<TokenStream> {
    all_fields
        .iter()
        .map(|f: &FieldDef| {
            let name = f.name();
            let is_in_create = create_fields.iter().any(|cf: &&FieldDef| cf.name() == name);

            if is_in_create {
                quote! { #name: dto.#name }
            } else if f.is_id {
                quote! { #name: uuid::Uuid::now_v7() }
            } else {
                quote! { #name: Default::default() }
            }
        })
        .collect()
}
