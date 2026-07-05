// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Joined read-model declarations from `#[join(...)]`.
//!
//! ```rust,ignore
//! #[join(airports as origin, on = origin_iata = iata, fields(
//!     lat as origin_lat: f64,
//!     lon as origin_lon: f64,
//!     city as origin_city: String
//! ))]
//! ```
//!
//! Each declaration contributes one `INNER JOIN` to the generated
//! `{Entity}View` read model: the joined table gets the declared alias,
//! the join condition matches a local entity column against a column of
//! the joined table, and every listed field is selected under its alias
//! with the declared Rust type (the macro cannot see the foreign
//! table's schema, so the type is part of the declaration).

use syn::{Attribute, Ident, Type};

/// One selected column of a joined table.
#[derive(Debug, Clone)]
pub struct JoinFieldDef {
    /// Column name on the joined table.
    pub source: String,

    /// Field/alias name in the generated view struct.
    pub alias: Ident,

    /// Rust type the column decodes to.
    pub ty: Type
}

/// One `#[join(...)]` declaration.
#[derive(Debug, Clone)]
pub struct JoinDef {
    /// Joined table name.
    pub table: String,

    /// SQL alias for the joined table.
    pub alias: String,

    /// Entity column on the local side of the join condition.
    pub local_column: String,

    /// Column of the joined table on the foreign side.
    pub foreign_column: String,

    /// Selected columns.
    pub fields: Vec<JoinFieldDef>
}

/// Parse all `#[join(...)]` attributes.
///
/// # Errors
///
/// Returns a `syn::Error` for malformed declarations: missing `as`
/// alias, missing/invalid `on`, empty or malformed `fields(...)`.
pub fn parse_join_attrs(attrs: &[Attribute]) -> syn::Result<Vec<JoinDef>> {
    let mut joins = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("join") {
            continue;
        }
        joins.push(attr.parse_args_with(parse_join_body)?);
    }

    Ok(joins)
}

/// Parse the body of one `#[join(...)]` attribute.
fn parse_join_body(input: syn::parse::ParseStream<'_>) -> syn::Result<JoinDef> {
    let table: Ident = input.parse()?;
    let _: syn::Token![as] = input.parse()?;
    let alias: Ident = input.parse()?;

    let _: syn::Token![,] = input.parse()?;
    let on_kw: Ident = input.parse()?;
    if on_kw != "on" {
        return Err(syn::Error::new(
            on_kw.span(),
            "expected `on = local_column = foreign_column`"
        ));
    }
    let _: syn::Token![=] = input.parse()?;
    let local: Ident = input.parse()?;
    let _: syn::Token![=] = input.parse()?;
    let foreign: Ident = input.parse()?;

    let _: syn::Token![,] = input.parse()?;
    let fields_kw: Ident = input.parse()?;
    if fields_kw != "fields" {
        return Err(syn::Error::new(
            fields_kw.span(),
            "expected `fields(source as alias: Type, ...)`"
        ));
    }
    let content;
    syn::parenthesized!(content in input);

    let mut fields = Vec::new();
    while !content.is_empty() {
        let source: Ident = content.parse()?;
        let _: syn::Token![as] = content.parse()?;
        let field_alias: Ident = content.parse()?;
        let _: syn::Token![:] = content.parse()?;
        let ty: Type = content.parse()?;
        fields.push(JoinFieldDef {
            source: source.to_string(),
            alias: field_alias,
            ty
        });
        if content.peek(syn::Token![,]) {
            let _: syn::Token![,] = content.parse()?;
        }
    }
    if fields.is_empty() {
        return Err(syn::Error::new(
            fields_kw.span(),
            "join requires at least one field in fields(...)"
        ));
    }

    Ok(JoinDef {
        table: table.to_string(),
        alias: alias.to_string(),
        local_column: local.to_string(),
        foreign_column: foreign.to_string(),
        fields
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    fn parse(tokens: proc_macro2::TokenStream) -> syn::Result<Vec<JoinDef>> {
        let input: syn::DeriveInput = syn::parse_quote! {
            #tokens
            pub struct Ticket {
                pub id: uuid::Uuid,
            }
        };
        parse_join_attrs(&input.attrs)
    }

    #[test]
    fn parses_full_declaration() {
        let joins = parse(quote! {
            #[join(airports as origin, on = origin_iata = iata, fields(
                lat as origin_lat: f64,
                city as origin_city: String
            ))]
        })
        .expect("valid join must parse");
        assert_eq!(joins.len(), 1);
        let j = &joins[0];
        assert_eq!(j.table, "airports");
        assert_eq!(j.alias, "origin");
        assert_eq!(j.local_column, "origin_iata");
        assert_eq!(j.foreign_column, "iata");
        assert_eq!(j.fields.len(), 2);
        assert_eq!(j.fields[0].source, "lat");
        assert_eq!(j.fields[0].alias.to_string(), "origin_lat");
    }

    #[test]
    fn parses_multiple_joins() {
        let joins = parse(quote! {
            #[join(airports as origin, on = origin_iata = iata, fields(lat as origin_lat: f64))]
            #[join(airports as dest, on = destination_iata = iata, fields(lat as destination_lat: f64))]
        })
        .expect("valid joins must parse");
        assert_eq!(joins.len(), 2);
        assert_eq!(joins[1].alias, "dest");
    }

    #[test]
    fn rejects_empty_fields() {
        let err = parse(quote! {
            #[join(airports as origin, on = origin_iata = iata, fields())]
        })
        .expect_err("empty fields must fail");
        assert!(err.to_string().contains("at least one field"));
    }

    #[test]
    fn rejects_missing_on() {
        let err = parse(quote! {
            #[join(airports as origin, at = origin_iata = iata, fields(lat as l: f64))]
        })
        .expect_err("missing on must fail");
        assert!(err.to_string().contains("expected `on"));
    }
}
