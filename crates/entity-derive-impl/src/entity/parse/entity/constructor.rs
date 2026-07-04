// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! `EntityDef` constructor implementation.
//!
//! This module provides [`EntityDef::from_derive_input`], the main entry point
//! for parsing entity definitions from proc-macro input.
//!
//! # Parsing Pipeline
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                  from_derive_input() Pipeline                       │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  DeriveInput                                                        │
//! │      │                                                              │
//! │      ├─► EntityAttrs::from_derive_input()  ──► Entity-level attrs   │
//! │      │                                                              │
//! │      ├─► Extract fields ──► FieldDef::from_field() ──► Vec<FieldDef>│
//! │      │                                                              │
//! │      ├─► parse_has_many_attrs()  ──► Vec<Ident> (relations)         │
//! │      │                                                              │
//! │      ├─► parse_projection_attrs() ──► Vec<ProjectionDef>            │
//! │      │                                                              │
//! │      ├─► parse_command_attrs() ──► Vec<CommandDef>                  │
//! │      │                                                              │
//! │      ├─► parse_api_attr() ──► ApiConfig                             │
//! │      │                                                              │
//! │      ├─► extract_doc_comments() ──► Option<String>                  │
//! │      │                                                              │
//! │      └─► Find #[id] field index ──► usize                           │
//! │                                                                     │
//! │      ▼                                                              │
//! │  EntityDef (combined result)                                        │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Validation
//!
//! The constructor validates:
//!
//! | Check | Error |
//! |-------|-------|
//! | Must be struct | "Entity can only be derived for structs" |
//! | Must have named fields | "Entity requires named fields" |
//! | Must have `#[id]` field | "Entity must have exactly one field with #[id]" |
//! | Required attributes | darling errors for missing `table` |
//!
//! # Error Handling
//!
//! Returns `darling::Result<EntityDef>` which provides:
//! - Accumulated errors (multiple errors reported at once)
//! - Span information for error messages
//! - Integration with proc-macro-error for nice diagnostics

use darling::FromDeriveInput;
use syn::DeriveInput;

use super::{
    super::{command::parse_command_attrs, field::FieldDef, returning::ReturningMode},
    CompositeIndexDef, EntityAttrs, EntityDef,
    helpers::{parse_api_attr, parse_has_many_attrs, parse_index_attrs},
    parse_projection_attrs,
    upsert::{UpsertAction, UpsertDef}
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

        let mut fields: Vec<FieldDef> = match &input.data {
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
        let command_defs = parse_command_attrs(&input.attrs).map_err(darling::Error::from)?;
        let api_config = parse_api_attr(&input.attrs);
        let indexes = parse_index_attrs(&input.attrs);
        let doc = extract_doc_comments(&input.attrs);

        let id_field_index = fields
            .iter()
            .position(super::super::field::FieldDef::is_id)
            .ok_or_else(|| {
                darling::Error::custom("Entity must have exactly one field with #[id] attribute")
                    .with_span(&input.ident)
            })?;

        let owner_count = fields.iter().filter(|f| f.storage.is_owner).count();
        if owner_count > 1 {
            return Err(
                darling::Error::custom("Entity can have at most one #[owner] field")
                    .with_span(&input.ident)
            );
        }
        if let Some(owner) = fields.iter().find(|f| f.storage.is_owner)
            && owner.is_id()
        {
            return Err(darling::Error::custom(
                "#[owner] cannot be combined with #[id]: the owner column scopes rows of another principal"
            )
            .with_span(&input.ident));
        }

        let version_fields: Vec<usize> = fields
            .iter()
            .enumerate()
            .filter(|(_, f)| f.storage.is_version)
            .map(|(i, _)| i)
            .collect();
        if version_fields.len() > 1 {
            return Err(
                darling::Error::custom("Entity can have at most one #[version] field")
                    .with_span(&input.ident)
            );
        }
        if let Some(&idx) = version_fields.first() {
            let ty_ok = matches!(
                &fields[idx].ty,
                syn::Type::Path(tp) if tp
                    .path
                    .segments
                    .last()
                    .is_some_and(|s| matches!(s.ident.to_string().as_str(), "i16" | "i32" | "i64"))
            );
            if !ty_ok {
                return Err(darling::Error::custom(
                    "#[version] requires an integer field (i16, i32 or i64)"
                )
                .with_span(&fields[idx].ident));
            }
            if fields[idx].column.default.is_none() {
                fields[idx].column.default = Some("0".to_string());
            }
        }

        if attrs.migrations.touch_updated_at
            && !fields.iter().any(|f| f.name_str() == "updated_at")
        {
            return Err(darling::Error::custom(
                "migrations(touch_updated_at) requires an `updated_at` field"
            )
            .with_span(&input.ident));
        }

        if let Some(upsert) = &attrs.upsert {
            validate_upsert(
                upsert,
                &fields,
                id_field_index,
                &indexes,
                &attrs.returning,
                input
            )?;
        }

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
            events: attrs.events.enabled,
            outbox: attrs.events.outbox,
            hooks: attrs.hooks,
            commands: attrs.commands,
            command_defs,
            policy: attrs.policy,
            streams: attrs.streams,
            transactions: attrs.transactions,
            api_config,
            doc,
            migrations: attrs.migrations.enabled,
            touch_updated_at: attrs.migrations.touch_updated_at,
            audit: attrs.migrations.audit,
            extensions: attrs.migrations.extensions,
            indexes,
            aggregate_root: attrs.aggregate_root,
            upsert: attrs.upsert
        })
    }
}

/// Validate `#[entity(upsert(...))]` configuration at parse time.
///
/// # Rules
///
/// | Rule | Rationale |
/// |------|-----------|
/// | At least one conflict column | `ON CONFLICT ()` is invalid SQL |
/// | Entity has `#[field(create)]` fields | `upsert` consumes the Create DTO |
/// | Every conflict column maps to a field | Typos surface at compile time |
/// | Conflict target carries a uniqueness guarantee | `ON CONFLICT` requires a unique index or constraint |
/// | `returning = "full"` | The returned entity must reflect the persisted row, which on the update path is the pre-existing one |
/// | `action = "update"` needs a non-conflict insert column | An empty `DO UPDATE SET` is invalid SQL |
fn validate_upsert(
    upsert: &UpsertDef,
    fields: &[FieldDef],
    id_field_index: usize,
    indexes: &[CompositeIndexDef],
    returning: &ReturningMode,
    input: &DeriveInput
) -> darling::Result<()> {
    let span = &input.ident;
    let conflict = upsert.conflict_columns();

    if conflict.is_empty() {
        return Err(darling::Error::custom(
            "upsert requires at least one conflict column, e.g. upsert(conflict = \"email\")"
        )
        .with_span(span));
    }

    if !fields.iter().any(|f| f.expose.create) {
        return Err(darling::Error::custom(
            "upsert requires at least one #[field(create)] field: the generated method takes the Create DTO"
        )
        .with_span(span));
    }

    if *returning != ReturningMode::Full {
        return Err(darling::Error::custom(
            "upsert requires returning = \"full\": on the update path the persisted row differs from the pre-built entity"
        )
        .with_span(span));
    }

    let column_names: Vec<String> = fields.iter().map(FieldDef::name_str).collect();
    for col in &conflict {
        if !column_names.iter().any(|c| c == col) {
            return Err(darling::Error::custom(format!(
                "upsert conflict column `{col}` does not match any entity column"
            ))
            .with_span(span));
        }
    }

    let guaranteed_unique = match conflict.as_slice() {
        [single] => {
            let id_column = fields[id_field_index].name_str();
            *single == id_column
                || fields
                    .iter()
                    .any(|f| f.name_str() == *single && f.column.unique)
                || unique_index_matches(indexes, &conflict)
        }
        _ => unique_index_matches(indexes, &conflict)
    };

    if !guaranteed_unique {
        return Err(darling::Error::custom(format!(
            "upsert conflict target ({}) has no uniqueness guarantee: mark the column with #[column(unique)] or declare unique_index({})",
            conflict.join(", "),
            conflict.join(", ")
        ))
        .with_span(span));
    }

    if upsert.action == UpsertAction::Update {
        let has_updatable_column = fields.iter().any(|f| {
            let col = f.name_str();
            !f.is_id() && !conflict.contains(&col)
        });
        if !has_updatable_column {
            return Err(darling::Error::custom(
                "upsert action = \"update\" needs at least one non-conflict column to update; use action = \"nothing\" instead"
            )
            .with_span(span));
        }
    }

    Ok(())
}

/// Whether a declared `unique_index(...)` covers exactly the conflict
/// columns (order-insensitive).
fn unique_index_matches(indexes: &[CompositeIndexDef], conflict: &[String]) -> bool {
    indexes.iter().any(|idx| {
        idx.unique
            && idx.columns.len() == conflict.len()
            && conflict.iter().all(|c| idx.columns.contains(c))
    })
}
