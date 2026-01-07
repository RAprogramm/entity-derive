// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Accessor methods for EntityDef.

use proc_macro2::Span;
use syn::Ident;

use super::{
    super::{api::ApiConfig, command::CommandDef, field::FieldDef},
    EntityDef
};

impl EntityDef {
    /// Get the primary key field marked with `#[id]`.
    ///
    /// This field is guaranteed to exist as it's validated during parsing.
    pub fn id_field(&self) -> &FieldDef {
        &self.fields[self.id_field_index]
    }

    /// Get fields to include in `CreateRequest` DTO.
    ///
    /// Returns fields where:
    /// - `#[field(create)]` is present
    /// - NOT marked with `#[id]` (IDs are auto-generated)
    /// - NOT marked with `#[auto]` (timestamps are auto-generated)
    /// - NOT marked with `#[field(skip)]`
    pub fn create_fields(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| f.in_create() && !f.is_id() && !f.is_auto())
            .collect()
    }

    /// Get fields to include in `UpdateRequest` DTO.
    ///
    /// Returns fields where:
    /// - `#[field(update)]` is present
    /// - NOT marked with `#[id]` (can't update primary key)
    /// - NOT marked with `#[auto]` (timestamps auto-update)
    /// - NOT marked with `#[field(skip)]`
    pub fn update_fields(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| f.in_update() && !f.is_id() && !f.is_auto())
            .collect()
    }

    /// Get fields to include in `Response` DTO.
    ///
    /// Returns fields where:
    /// - `#[field(response)]` is present, OR
    /// - `#[id]` is present (IDs always in response)
    /// - NOT marked with `#[field(skip)]`
    pub fn response_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.in_response()).collect()
    }

    /// Get all fields for Row and Insertable structs.
    ///
    /// These database-layer structs include ALL fields from the
    /// entity, regardless of DTO inclusion settings.
    pub fn all_fields(&self) -> &[FieldDef] {
        &self.fields
    }

    /// Get fields with `#[belongs_to]` relations.
    ///
    /// Returns fields that are foreign keys to other entities.
    /// Used to generate relation methods in the repository.
    pub fn relation_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.is_relation()).collect()
    }

    /// Get fields with `#[filter]` attribute.
    ///
    /// Returns fields that can be used in query filtering.
    /// Used to generate the Query struct and query method.
    pub fn filter_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.has_filter()).collect()
    }

    /// Check if this entity has any filterable fields.
    pub fn has_filters(&self) -> bool {
        self.fields.iter().any(|f| f.has_filter())
    }

    /// Get has-many relations defined via `#[has_many(Entity)]`.
    ///
    /// Returns entity identifiers for one-to-many relationships.
    /// Used to generate collection methods in the repository.
    pub fn has_many_relations(&self) -> &[Ident] {
        &self.has_many
    }

    /// Get the entity name as an identifier.
    pub fn name(&self) -> &Ident {
        &self.ident
    }

    /// Get the entity name as a string.
    pub fn name_str(&self) -> String {
        self.ident.to_string()
    }

    /// Get the fully qualified table name with schema.
    pub fn full_table_name(&self) -> String {
        format!("{}.{}", self.schema, self.table)
    }

    /// Create a new identifier with prefix and/or suffix.
    ///
    /// Used to generate related type names following naming conventions.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // For entity "User":
    /// entity.ident_with("Create", "Request") // CreateUserRequest
    /// entity.ident_with("Update", "Request") // UpdateUserRequest
    /// entity.ident_with("", "Response")      // UserResponse
    /// entity.ident_with("", "Row")           // UserRow
    /// entity.ident_with("Insertable", "")    // InsertableUser
    /// entity.ident_with("", "Repository")    // UserRepository
    /// ```
    pub fn ident_with(&self, prefix: &str, suffix: &str) -> Ident {
        Ident::new(
            &format!("{}{}{}", prefix, self.name_str(), suffix),
            Span::call_site()
        )
    }

    /// Get the error type for repository implementation.
    pub fn error_type(&self) -> &syn::Path {
        &self.error
    }

    /// Check if soft delete is enabled for this entity.
    pub fn is_soft_delete(&self) -> bool {
        self.soft_delete
    }

    /// Check if lifecycle events should be generated.
    pub fn has_events(&self) -> bool {
        self.events
    }

    /// Check if lifecycle hooks trait should be generated.
    pub fn has_hooks(&self) -> bool {
        self.hooks
    }

    /// Check if CQRS-style commands should be generated.
    pub fn has_commands(&self) -> bool {
        self.commands
    }

    /// Get command definitions.
    pub fn command_defs(&self) -> &[CommandDef] {
        &self.command_defs
    }

    /// Check if authorization policy should be generated.
    pub fn has_policy(&self) -> bool {
        self.policy
    }

    /// Check if real-time streaming should be enabled.
    pub fn has_streams(&self) -> bool {
        self.streams
    }

    /// Check if transaction support should be generated.
    pub fn has_transactions(&self) -> bool {
        self.transactions
    }

    /// Check if API generation is enabled.
    #[allow(dead_code)]
    pub fn has_api(&self) -> bool {
        self.api_config.is_enabled()
    }

    /// Get API configuration.
    #[allow(dead_code)]
    pub fn api_config(&self) -> &ApiConfig {
        &self.api_config
    }

    /// Get the documentation comment if present.
    #[must_use]
    #[allow(dead_code)]
    pub fn doc(&self) -> Option<&str> {
        self.doc.as_deref()
    }
}
