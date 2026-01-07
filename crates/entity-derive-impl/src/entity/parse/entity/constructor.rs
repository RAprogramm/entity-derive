// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! EntityDef constructor (from_derive_input).

use darling::FromDeriveInput;
use syn::DeriveInput;

use super::{
    super::{command::parse_command_attrs, field::FieldDef},
    EntityAttrs, EntityDef,
    helpers::{parse_api_attr, parse_has_many_attrs},
    parse_projection_attrs
};
use crate::utils::docs::extract_doc_comments;

impl EntityDef {
    /// Parse entity definition from syn's `DeriveInput`.
    ///
    /// This is the main entry point for parsing. It:
    ///
    /// 1. Parses entity-level attributes using darling
    /// 2. Extracts all named fields from the struct
    /// 3. Parses field-level attributes for each field
    /// 4. Combines everything into an `EntityDef`
    ///
    /// # Arguments
    ///
    /// * `input` - Parsed derive input from syn
    ///
    /// # Returns
    ///
    /// `Ok(EntityDef)` on success, or `Err` with darling errors.
    ///
    /// # Errors
    ///
    /// - Missing `table` attribute
    /// - Applied to non-struct (enum, union)
    /// - Applied to tuple struct or unit struct
    /// - Invalid attribute values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// pub fn derive(input: TokenStream) -> TokenStream {
    ///     let input = parse_macro_input!(input as DeriveInput);
    ///
    ///     match EntityDef::from_derive_input(&input) {
    ///         Ok(entity) => generate(entity),
    ///         Err(err) => err.write_errors().into()
    ///     }
    /// }
    /// ```
    pub fn from_derive_input(input: &DeriveInput) -> darling::Result<Self> {
        let attrs = EntityAttrs::from_derive_input(input)?;

        let fields: Vec<FieldDef> = match &input.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(named) => named
                    .named
                    .iter()
                    .map(FieldDef::from_field)
                    .collect::<darling::Result<Vec<_>>>()?,
                _ => {
                    return Err(darling::Error::custom("Entity requires named fields")
                        .with_span(&input.ident));
                }
            },
            _ => {
                return Err(
                    darling::Error::custom("Entity can only be derived for structs")
                        .with_span(&input.ident)
                );
            }
        };

        let has_many = parse_has_many_attrs(&input.attrs);
        let projections = parse_projection_attrs(&input.attrs);
        let command_defs = parse_command_attrs(&input.attrs);
        let api_config = parse_api_attr(&input.attrs);
        let doc = extract_doc_comments(&input.attrs);

        let id_field_index = fields.iter().position(|f| f.is_id()).ok_or_else(|| {
            darling::Error::custom("Entity must have exactly one field with #[id] attribute")
                .with_span(&input.ident)
        })?;

        Ok(Self {
            ident: attrs.ident,
            vis: attrs.vis,
            table: attrs.table,
            schema: attrs.schema,
            sql: attrs.sql,
            dialect: attrs.dialect,
            uuid: attrs.uuid,
            error: attrs.error,
            fields,
            id_field_index,
            has_many,
            projections,
            soft_delete: attrs.soft_delete,
            returning: attrs.returning,
            events: attrs.events,
            hooks: attrs.hooks,
            commands: attrs.commands,
            command_defs,
            policy: attrs.policy,
            streams: attrs.streams,
            transactions: attrs.transactions,
            api_config,
            doc
        })
    }
}
