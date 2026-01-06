// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Generation context for PostgreSQL repository.
//!
//! Contains the [`Context`] struct that precomputes all identifiers and SQL
//! fragments needed for method generation.

use quote::format_ident;

use super::helpers::join_columns;
use crate::entity::parse::{DatabaseDialect, EntityDef, ReturningMode};

/// Context for PostgreSQL code generation.
///
/// Precomputes all identifiers and SQL fragments needed for method generation.
/// This avoids repeated computation and provides a clean API for method
/// generators.
///
/// # Fields
///
/// | Field | Description |
/// |-------|-------------|
/// | `entity` | Reference to the parsed entity definition |
/// | `dialect` | Database dialect (always Postgres here) |
/// | `trait_name` | Repository trait name (e.g., `UserRepository`) |
/// | `entity_name` | Entity struct name (e.g., `User`) |
/// | `row_name` | Row struct name (e.g., `UserRow`) |
/// | `table` | Full table name with schema (e.g., `public.users`) |
/// | `columns_str` | Comma-separated column names |
/// | `placeholders_str` | Comma-separated placeholders (`$1, $2, ...`) |
pub struct Context<'a> {
    /// Reference to the parsed entity definition.
    pub entity: &'a EntityDef,

    /// Database dialect for SQL generation.
    pub dialect: DatabaseDialect,

    /// Repository trait name (e.g., `UserRepository`).
    pub trait_name: syn::Ident,

    /// Entity struct name (e.g., `User`).
    pub entity_name: &'a syn::Ident,

    /// Row struct name for database mapping (e.g., `UserRow`).
    pub row_name: syn::Ident,

    /// Insertable struct name (e.g., `InsertableUser`).
    pub insertable_name: syn::Ident,

    /// Create DTO name (e.g., `CreateUserRequest`).
    pub create_dto: syn::Ident,

    /// Update DTO name (e.g., `UpdateUserRequest`).
    pub update_dto: syn::Ident,

    /// Full table name with schema (e.g., `public.users`).
    pub table: String,

    /// Primary key field name.
    pub id_name: &'a syn::Ident,

    /// Primary key field type.
    pub id_type: &'a syn::Type,

    /// Comma-separated column names for SELECT/INSERT.
    pub columns_str: String,

    /// Comma-separated placeholders for INSERT ($1, $2, ...).
    pub placeholders_str: String,

    /// Whether soft delete is enabled.
    pub soft_delete: bool,

    /// RETURNING clause mode.
    pub returning: ReturningMode
}

impl<'a> Context<'a> {
    /// Create a new generation context from an entity definition.
    pub fn new(entity: &'a EntityDef) -> Self {
        let id_field = entity.id_field();
        let fields = entity.all_fields();
        let dialect = entity.dialect;

        Self {
            entity,
            dialect,
            trait_name: format_ident!("{}Repository", entity.name()),
            entity_name: entity.name(),
            row_name: entity.ident_with("", "Row"),
            insertable_name: entity.ident_with("Insertable", ""),
            create_dto: entity.ident_with("Create", "Request"),
            update_dto: entity.ident_with("Update", "Request"),
            table: entity.full_table_name(),
            id_name: id_field.name(),
            id_type: id_field.ty(),
            columns_str: join_columns(fields),
            placeholders_str: dialect.placeholders(fields.len()),
            soft_delete: entity.is_soft_delete(),
            returning: entity.returning.clone()
        }
    }
}
