// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Accessor methods for `EntityDef`.
//!
//! This module provides getter methods for accessing `EntityDef` fields and
//! computed values. Methods are organized by purpose: field access, naming
//! helpers, and feature flags.
//!
//! # Method Categories
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    EntityDef Accessors                              │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Field Access            Naming                Feature Checks       │
//! │  ├── id_field()          ├── name()            ├── is_soft_delete() │
//! │  ├── create_fields()     ├── name_str()        ├── has_events()     │
//! │  ├── update_fields()     ├── full_table_name() ├── has_hooks()      │
//! │  ├── response_fields()   ├── table_name()      ├── has_commands()   │
//! │  ├── all_fields()        └── ident_with()      ├── has_policy()     │
//! │  ├── relation_fields()                         ├── has_streams()    │
//! │  └── filter_fields()                           ├── has_transactions()│
//! │                                                ├── has_api()        │
//! │  Configuration                                 └── has_filters()    │
//! │  ├── error_type()                                                   │
//! │  ├── api_config()                                                   │
//! │  └── command_defs()                                                 │
//! │  └── doc()                                                          │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Field Category Methods
//!
//! These methods return filtered field collections for DTO generation:
//!
//! | Method | Returns | Used For |
//! |--------|---------|----------|
//! | `id_field()` | Primary key field | All DTOs and queries |
//! | `create_fields()` | `#[field(create)]` fields | `CreateRequest` DTO |
//! | `update_fields()` | `#[field(update)]` fields | `UpdateRequest` DTO |
//! | `response_fields()` | `#[field(response)]` + ID | `Response` DTO |
//! | `all_fields()` | All fields | `Row`, `Insertable` |
//! | `relation_fields()` | `#[belongs_to]` fields | Relation methods |
//! | `filter_fields()` | `#[filter]` fields | Query struct |
//!
//! # Naming Methods
//!
//! | Method | Example | Result |
//! |--------|---------|--------|
//! | `name()` | `User` | `Ident("User")` |
//! | `name_str()` | `User` | `"User"` |
//! | `table_name()` | `users` | `"users"` |
//! | `full_table_name()` | (no schema) → `"users"`, (schema="core") → `"core.users"` |
//! | `ident_with("Create", "Request")` | `User` | `Ident("CreateUserRequest")` |

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
    /// Get the `#[owner]` field, if any.
    ///
    /// Marks the column carrying the owning principal's id and enables
    /// generated row-level scoped repository methods.
    #[must_use]
    pub fn owner_field(&self) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.storage.is_owner)
    }

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
    /// Get the `#[version]` optimistic-locking field, if any.
    #[must_use]
    pub fn version_field(&self) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.storage.is_version)
    }

    /// Get all fields marked `#[sort]`.
    #[must_use]
    pub fn sort_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.sortable).collect()
    }

    /// Check if the entity has any `#[sort]` fields.
    #[must_use]
    pub fn has_sort_fields(&self) -> bool {
        self.fields.iter().any(|f| f.sortable)
    }

    pub fn has_filters(&self) -> bool {
        self.fields
            .iter()
            .any(super::super::field::FieldDef::has_filter)
    }

    /// Get has-many relations defined via `#[has_many(Entity)]`.
    ///
    /// Returns entity identifiers for one-to-many relationships.
    /// Used to generate collection methods in the repository.
    pub fn has_many_relations(&self) -> &[super::HasManyDef] {
        &self.has_many
    }

    /// Get the entity name as an identifier.
    pub const fn name(&self) -> &Ident {
        &self.ident
    }

    /// Get the entity name as a string.
    pub fn name_str(&self) -> String {
        self.ident.to_string()
    }

    /// Get the fully qualified table name with schema prefix.
    ///
    /// When `schema` is not specified (empty string), returns just the
    /// table name. When `schema` is specified (any non-empty value),
    /// returns `"schema.table"` format.
    ///
    /// # Examples
    ///
    /// ```text
    /// // No schema specified
    /// #[entity(table = "users")]
    /// // → "users"
    ///
    /// // Schema explicitly set to "public"
    /// #[entity(table = "users", schema = "public")]
    /// // → "public.users"
    ///
    /// // Custom schema
    /// #[entity(table = "users", schema = "core")]
    /// // → "core.users"
    /// ```
    pub fn full_table_name(&self) -> String {
        match self.schema.as_str() {
            "" => self.table.clone(),
            other => format!("{other}.{}", self.table)
        }
    }

    /// Get just the table name without schema prefix.
    ///
    /// This is useful when you need the raw table name regardless of
    /// schema configuration.
    ///
    /// # Examples
    ///
    /// ```text
    /// // For entity with table = "users" and schema = "core"
    /// entity.table_name()  // → "users"
    /// entity.full_table_name()  // → "core.users"
    /// ```
    #[allow(dead_code)]
    pub fn table_name(&self) -> &str {
        &self.table
    }

    /// Build a fully qualified table name for an arbitrary table identifier.
    ///
    /// Applies the entity's schema configuration to the given table name.
    /// When schema is empty, returns just the table name. When schema is
    /// set, returns `"schema.table"` format.
    ///
    /// # Arguments
    ///
    /// * `table` - The table name to qualify with schema prefix
    ///
    /// # Examples
    ///
    /// ```text
    /// // No schema: entity.schema = ""
    /// entity.full_table_name_for("users")  // → "users"
    ///
    /// // Schema set: entity.schema = "core"
    /// entity.full_table_name_for("users")  // → "core.users"
    /// ```
    pub fn full_table_name_for(&self, table: &str) -> String {
        match self.schema.as_str() {
            "" => table.to_string(),
            other => format!("{other}.{table}")
        }
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
    pub const fn error_type(&self) -> &syn::Path {
        &self.error
    }

    /// Check if soft delete is enabled for this entity.
    pub const fn is_soft_delete(&self) -> bool {
        self.soft_delete
    }

    /// Check if lifecycle events should be generated.
    pub const fn has_events(&self) -> bool {
        self.events
    }

    /// Check if transactional-outbox delivery is enabled.
    #[must_use]
    pub const fn has_outbox(&self) -> bool {
        self.outbox
    }

    /// Check if lifecycle hooks trait should be generated.
    pub const fn has_hooks(&self) -> bool {
        self.hooks
    }

    /// Check if CQRS-style commands should be generated.
    pub const fn has_commands(&self) -> bool {
        self.commands
    }

    /// Get command definitions.
    pub fn command_defs(&self) -> &[CommandDef] {
        &self.command_defs
    }

    /// Check if authorization policy should be generated.
    pub const fn has_policy(&self) -> bool {
        self.policy
    }

    /// Check if real-time streaming should be enabled.
    pub const fn has_streams(&self) -> bool {
        self.streams
    }

    /// Check if transaction support should be generated.
    pub const fn has_transactions(&self) -> bool {
        self.transactions
    }

    /// Check if API generation is enabled.
    #[allow(dead_code)]
    pub const fn has_api(&self) -> bool {
        self.api_config.is_enabled()
    }

    /// Get API configuration.
    #[allow(dead_code)]
    pub const fn api_config(&self) -> &ApiConfig {
        &self.api_config
    }

    /// Check if aggregate root pattern is enabled.
    #[must_use]
    pub const fn is_aggregate_root(&self) -> bool {
        self.aggregate_root
    }

    /// Get fields with `#[column(unique)]` or `#[column(index)]`.
    ///
    /// These fields are used to generate lookup methods
    /// (`find_by_{field}`, `exists_by_{field}`) in the repository.
    ///
    /// # Returns
    ///
    /// A vector of field references where the column configuration
    /// has `unique` set to `true` or `index` is `Some(_)`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // For entity with:
    /// //   #[column(unique)] pub email: String
    /// //   #[column(index)] pub status: String
    /// //   pub name: String
    /// //
    /// // lookup_fields() returns [email_field, status_field]
    /// let fields = entity.lookup_fields();
    /// ```
    pub fn lookup_fields(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| f.column.unique || f.column.index.is_some())
            .collect()
    }

    /// Get the documentation comment if present.
    #[must_use]
    #[allow(dead_code)]
    pub fn doc(&self) -> Option<&str> {
        self.doc.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: syn::DeriveInput = syn::parse_quote!(#tokens);
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn full_table_name_empty_schema() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        assert_eq!(entity.full_table_name(), "users");
    }

    #[test]
    fn full_table_name_public_schema() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", schema = "public")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        assert_eq!(entity.full_table_name(), "public.users");
    }

    #[test]
    fn full_table_name_custom_schema() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", schema = "core")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        assert_eq!(entity.full_table_name(), "core.users");
    }

    #[test]
    fn full_table_name_tenants_schema() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders", schema = "tenants")]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        assert_eq!(entity.full_table_name(), "tenants.orders");
    }

    #[test]
    fn table_name_returns_raw_table() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", schema = "core")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        assert_eq!(entity.table_name(), "users");
    }

    #[test]
    fn table_name_without_schema() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "products")]
            pub struct Product {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        assert_eq!(entity.table_name(), "products");
    }
}
