// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! State-machine transition declarations from `#[transition(...)]`.
//!
//! ```rust,ignore
//! #[transition(created -> accepted, sets(courier_id, ticket_id))]
//! #[transition(accepted -> in_transit)]
//! #[transition(created | accepted -> cancelled)]
//! ```
//!
//! Source and target statuses are snake_case idents mapping to
//! PascalCase variants of the entity's `status` field enum. `sets(...)`
//! lists additional entity columns patched together with the status.

use convert_case::{Case, Casing};
use syn::{Attribute, Ident};

/// One `#[transition(...)]` declaration.
#[derive(Debug, Clone)]
pub struct TransitionDef {
    /// Allowed source statuses (snake_case, as written).
    pub sources: Vec<String>,

    /// Target status (snake_case, as written).
    pub target: String,

    /// Additional entity columns patched by the transition.
    pub sets: Vec<String>
}

impl TransitionDef {
    /// PascalCase enum variant for a snake_case status ident.
    #[must_use]
    pub fn variant(status: &str) -> String {
        status.to_case(Case::Pascal)
    }

    /// Generated method name: `transition_to_{target}`.
    #[must_use]
    pub fn method_name(&self) -> String {
        format!("transition_to_{}", self.target)
    }
}

/// Parse all `#[transition(...)]` attributes.
///
/// # Errors
///
/// Returns a `syn::Error` for malformed declarations: missing `->`,
/// empty sources, or an unknown option.
pub fn parse_transition_attrs(attrs: &[Attribute]) -> syn::Result<Vec<TransitionDef>> {
    let mut transitions = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("transition") {
            continue;
        }
        transitions.push(attr.parse_args_with(parse_transition_body)?);
    }

    Ok(transitions)
}

/// Parse the body of one `#[transition(...)]` attribute.
fn parse_transition_body(input: syn::parse::ParseStream<'_>) -> syn::Result<TransitionDef> {
    let mut sources = Vec::new();
    let first: Ident = input.parse()?;
    sources.push(first.to_string());
    while input.peek(syn::Token![|]) {
        let _: syn::Token![|] = input.parse()?;
        let next: Ident = input.parse()?;
        sources.push(next.to_string());
    }

    let _: syn::Token![->] = input.parse()?;
    let target: Ident = input.parse()?;

    let mut sets = Vec::new();
    if input.peek(syn::Token![,]) {
        let _: syn::Token![,] = input.parse()?;
        let sets_kw: Ident = input.parse()?;
        if sets_kw != "sets" {
            return Err(syn::Error::new(
                sets_kw.span(),
                "unknown transition option; expected `sets(column, ...)`"
            ));
        }
        let content;
        syn::parenthesized!(content in input);
        while !content.is_empty() {
            let col: Ident = content.parse()?;
            sets.push(col.to_string());
            if content.peek(syn::Token![,]) {
                let _: syn::Token![,] = content.parse()?;
            }
        }
        if sets.is_empty() {
            return Err(syn::Error::new(
                sets_kw.span(),
                "sets(...) requires at least one column"
            ));
        }
    }

    Ok(TransitionDef {
        sources,
        target: target.to_string(),
        sets
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    fn parse(tokens: proc_macro2::TokenStream) -> syn::Result<Vec<TransitionDef>> {
        let input: syn::DeriveInput = syn::parse_quote! {
            #tokens
            pub struct Parcel {
                pub id: uuid::Uuid,
            }
        };
        parse_transition_attrs(&input.attrs)
    }

    #[test]
    fn parses_simple_transition() {
        let t = parse(quote! {
            #[transition(accepted -> in_transit)]
        })
        .expect("must parse");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].sources, vec!["accepted"]);
        assert_eq!(t[0].target, "in_transit");
        assert!(t[0].sets.is_empty());
        assert_eq!(t[0].method_name(), "transition_to_in_transit");
    }

    #[test]
    fn parses_multi_source_and_sets() {
        let t = parse(quote! {
            #[transition(created | accepted -> cancelled)]
            #[transition(created -> accepted, sets(courier_id, ticket_id))]
        })
        .expect("must parse");
        assert_eq!(t[0].sources, vec!["created", "accepted"]);
        assert_eq!(t[1].sets, vec!["courier_id", "ticket_id"]);
    }

    #[test]
    fn variant_is_pascal_case() {
        assert_eq!(TransitionDef::variant("in_transit"), "InTransit");
        assert_eq!(TransitionDef::variant("created"), "Created");
    }

    #[test]
    fn rejects_unknown_option() {
        let err = parse(quote! {
            #[transition(created -> accepted, with(courier_id))]
        })
        .expect_err("unknown option must fail");
        assert!(err.to_string().contains("expected `sets"));
    }

    #[test]
    fn rejects_empty_sets() {
        let err = parse(quote! {
            #[transition(created -> accepted, sets())]
        })
        .expect_err("empty sets must fail");
        assert!(err.to_string().contains("at least one column"));
    }
}
