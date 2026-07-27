// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Participant scopes declared with `#[scope(...)]`.
//!
//! "Rows where this user takes part in any role" is an OR over several
//! columns holding the same kind of value, and it is the one query
//! shape that otherwise forces raw SQL:
//!
//! ```rust,ignore
//! #[scope(involving: requester_id | subject_id)]
//! #[scope(participating: sender_id | recipient_id | courier_id, within = parcel_id)]
//! ```
//!
//! The first generates `list_involving(value, limit, offset)`; the
//! second narrows the OR group to one parent first, generating
//! `list_participating(parcel_id, value, limit, offset)`.

use syn::{Attribute, Ident};

/// One `#[scope(...)]` declaration.
#[derive(Debug, Clone)]
pub struct ScopeDef {
    /// Scope name, used for the generated method: `list_{name}`.
    pub name:    String,
    /// Columns OR-ed against the bound value; at least two, since one
    /// column is what a plain lookup already covers.
    pub columns: Vec<String>,
    /// Optional column the scope is narrowed by first, AND-ed before
    /// the OR group.
    pub within:  Option<String>
}

impl ScopeDef {
    /// Generated method name.
    #[must_use]
    pub fn method_name(&self) -> String {
        format!("list_{}", self.name)
    }
}

/// Parse every `#[scope(...)]` attribute on the entity.
///
/// # Errors
///
/// Returns a `syn::Error` for a missing name separator, fewer than two
/// OR-ed columns, or an unknown option.
pub fn parse_scope_attrs(attrs: &[Attribute]) -> syn::Result<Vec<ScopeDef>> {
    let mut scopes = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("scope") {
            continue;
        }
        scopes.push(attr.parse_args_with(parse_scope_body)?);
    }

    Ok(scopes)
}

/// Parse the body of one `#[scope(...)]` attribute.
fn parse_scope_body(input: syn::parse::ParseStream<'_>) -> syn::Result<ScopeDef> {
    let name: Ident = input.parse()?;
    input.parse::<syn::Token![:]>().map_err(|_| {
        syn::Error::new(
            name.span(),
            "scope needs a name followed by `:` and the columns, for example \
             `#[scope(involving: requester_id | subject_id)]`"
        )
    })?;

    let mut columns = Vec::new();
    let first: Ident = input.parse()?;
    columns.push(first.to_string());
    while input.peek(syn::Token![|]) {
        input.parse::<syn::Token![|]>()?;
        let next: Ident = input.parse()?;
        columns.push(next.to_string());
    }

    if columns.len() < 2 {
        return Err(syn::Error::new(
            first.span(),
            "a scope ORs at least two columns; one column is what a lookup already does"
        ));
    }

    let mut within = None;
    if input.peek(syn::Token![,]) {
        input.parse::<syn::Token![,]>()?;
        let option: Ident = input.parse()?;
        if option != "within" {
            return Err(syn::Error::new(
                option.span(),
                "unknown scope option; expected `within = column`"
            ));
        }
        input.parse::<syn::Token![=]>()?;
        let column: Ident = input.parse()?;
        within = Some(column.to_string());
    }

    Ok(ScopeDef {
        name: name.to_string(),
        columns,
        within
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    fn parse(tokens: proc_macro2::TokenStream) -> syn::Result<Vec<ScopeDef>> {
        let input: syn::DeriveInput = syn::parse_quote! {
            #tokens
            pub struct Dispute {
                pub id: uuid::Uuid,
            }
        };
        parse_scope_attrs(&input.attrs)
    }

    #[test]
    fn parses_a_plain_or_group() {
        let scopes = parse(quote! { #[scope(involving: requester_id | subject_id)] })
            .expect("the declaration is valid");
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].name, "involving");
        assert_eq!(scopes[0].columns, vec!["requester_id", "subject_id"]);
        assert!(scopes[0].within.is_none());
        assert_eq!(scopes[0].method_name(), "list_involving");
    }

    #[test]
    fn parses_a_narrowed_group() {
        let scopes = parse(quote! {
            #[scope(participating: sender_id | recipient_id | courier_id, within = parcel_id)]
        })
        .expect("the declaration is valid");
        assert_eq!(scopes[0].columns.len(), 3);
        assert_eq!(scopes[0].within.as_deref(), Some("parcel_id"));
    }

    #[test]
    fn a_single_column_is_rejected() {
        let err = parse(quote! { #[scope(involving: requester_id)] })
            .expect_err("one column is not a scope");
        assert!(err.to_string().contains("at least two"), "{err}");
    }

    #[test]
    fn a_missing_separator_is_rejected() {
        let err = parse(quote! { #[scope(involving requester_id | subject_id)] })
            .expect_err("the name has to be followed by a colon");
        assert!(err.to_string().contains('`'), "{err}");
    }

    #[test]
    fn an_unknown_option_is_rejected() {
        let err = parse(quote! { #[scope(involving: a | b, ordered = c)] })
            .expect_err("only `within` is understood");
        assert!(err.to_string().contains("within"), "{err}");
    }

    #[test]
    fn no_attribute_yields_no_scopes() {
        let scopes = parse(quote! { #[entity(table = "disputes")] }).expect("nothing to parse");
        assert!(scopes.is_empty());
    }
}
