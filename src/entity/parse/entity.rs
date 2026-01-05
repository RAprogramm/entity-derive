// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Entity-level attribute parsing.
//!
//! This module handles parsing of entity-level attributes using darling,
//! and provides the main [`EntityDef`] structure used by all code generators.

use convert_case::{Case, Casing};
use darling::FromDeriveInput;
use proc_macro2::Span;
use syn::{Attribute, DeriveInput, Ident, Visibility};

use super::{
    dialect::DatabaseDialect, field::FieldDef, returning::ReturningMode, sql_level::SqlLevel,
    uuid_version::UuidVersion
};

/// Parse `#[has_many(Entity)]` attributes from struct attributes.
///
/// Returns a vector of related entity identifiers.
fn parse_has_many_attrs(attrs: &[Attribute]) -> Vec<Ident> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("has_many"))
        .filter_map(|attr| attr.parse_args::<Ident>().ok())
        .collect()
}

/// A projection definition parsed from `#[projection(Name: field1, field2)]`.
#[derive(Debug, Clone)]
pub struct ProjectionDef {
    /// Projection name (e.g., `Public`, `Admin`).
    pub name:   Ident,
    /// List of field names to include.
    pub fields: Vec<Ident>
}

/// Parse `#[projection(Name: field1, field2, ...)]` attributes.
///
/// Returns a vector of projection definitions.
fn parse_projection_attrs(attrs: &[Attribute]) -> Vec<ProjectionDef> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("projection"))
        .filter_map(|attr| {
            attr.parse_args_with(|input: syn::parse::ParseStream<'_>| {
                let name: Ident = input.parse()?;
                let _: syn::Token![:] = input.parse()?;
                let fields =
                    syn::punctuated::Punctuated::<Ident, syn::Token![,]>::parse_terminated(input)?;
                Ok(ProjectionDef {
                    name,
                    fields: fields.into_iter().collect()
                })
            })
            .ok()
        })
        .collect()
}

/// Default error type path for SQL implementations.
///
/// Used when no custom error type is specified.
fn default_error_type() -> syn::Path {
    syn::parse_str("sqlx::Error").expect("valid path")
}

/// Entity-level attributes parsed from `#[entity(...)]`.
///
/// This is an internal struct used by darling for parsing.
/// The public API uses [`EntityDef`] which combines these
/// attributes with parsed field definitions.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(entity), supports(struct_named))]
struct EntityAttrs {
    /// Struct identifier (e.g., `User`).
    ident: Ident,

    /// Struct visibility (e.g., `pub`, `pub(crate)`).
    vis: Visibility,

    /// Database table name.
    ///
    /// This is a required attribute with no default value.
    /// The macro will fail with a clear error if not provided.
    table: String,

    /// Database schema name.
    ///
    /// Defaults to `"public"` if not specified.
    #[darling(default = "default_schema")]
    schema: String,

    /// SQL generation level.
    ///
    /// Defaults to [`SqlLevel::Full`] if not specified.
    #[darling(default)]
    sql: SqlLevel,

    /// Database dialect.
    ///
    /// Defaults to [`DatabaseDialect::Postgres`] if not specified.
    #[darling(default)]
    dialect: DatabaseDialect,

    /// UUID version for ID generation.
    ///
    /// Defaults to [`UuidVersion::V7`] if not specified.
    #[darling(default)]
    uuid: UuidVersion,

    /// Custom error type for repository implementation.
    ///
    /// Defaults to `sqlx::Error` if not specified.
    /// The custom type must implement `From<sqlx::Error>`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", error = "AppError")]
    /// #[entity(table = "users", error = "crate::errors::DbError")]
    /// ```
    #[darling(default = "default_error_type")]
    error: syn::Path,

    /// Enable soft delete for this entity.
    ///
    /// When enabled, the entity must have a `deleted_at: Option<DateTime<Utc>>`
    /// field. The `delete` method will set this timestamp instead of removing
    /// the row, and all queries will automatically filter out deleted records.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", soft_delete)]
    /// pub struct User {
    ///     #[id]
    ///     pub id: Uuid,
    ///     pub deleted_at: Option<DateTime<Utc>>,
    /// }
    /// ```
    #[darling(default)]
    soft_delete: bool,

    /// RETURNING clause mode for INSERT/UPDATE operations.
    ///
    /// Controls what data is fetched back from the database:
    /// - `full` (default): Use `RETURNING *` to get all fields
    /// - `id`: Use `RETURNING id` to get only the primary key
    /// - `none`: No RETURNING clause, return pre-built entity
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", returning = "full")]
    /// #[entity(table = "logs", returning = "none")]
    /// ```
    #[darling(default)]
    returning: ReturningMode
}

/// Returns the default schema name.
///
/// Used by darling for the `schema` attribute default.
fn default_schema() -> String {
    "public".to_string()
}

/// Complete parsed entity definition.
///
/// This is the main data structure passed to all code generators.
/// It contains both entity-level metadata and all field definitions.
///
/// # Construction
///
/// Create via [`EntityDef::from_derive_input`]:
///
/// ```rust,ignore
/// let entity = EntityDef::from_derive_input(&input)?;
/// ```
///
/// # Field Access
///
/// Use the provided methods to access fields by category:
///
/// ```rust,ignore
/// // All fields for Row/Insertable
/// let all = entity.all_fields();
///
/// // Fields for specific DTOs
/// let create_fields = entity.create_fields();
/// let update_fields = entity.update_fields();
/// let response_fields = entity.response_fields();
///
/// // Primary key field
/// let id = entity.id_field().expect("must have #[id]");
/// ```
#[derive(Debug)]
pub struct EntityDef {
    /// Struct identifier (e.g., `User`).
    pub ident: Ident,

    /// Struct visibility.
    ///
    /// Propagated to all generated types so they have the same
    /// visibility as the source entity.
    pub vis: Visibility,

    /// Database table name (e.g., `"users"`).
    pub table: String,

    /// Database schema name (e.g., `"public"`, `"core"`).
    pub schema: String,

    /// SQL generation level controlling what code is generated.
    pub sql: SqlLevel,

    /// Database dialect for code generation.
    pub dialect: DatabaseDialect,

    /// UUID version for ID generation.
    pub uuid: UuidVersion,

    /// Custom error type for repository implementation.
    ///
    /// Defaults to `sqlx::Error`. Custom types must implement
    /// `From<sqlx::Error>` for the `?` operator to work.
    pub error: syn::Path,

    /// All field definitions from the struct.
    pub fields: Vec<FieldDef>,

    /// Has-many relations defined via `#[has_many(Entity)]`.
    ///
    /// Each entry is the related entity name.
    pub has_many: Vec<Ident>,

    /// Projections defined via `#[projection(Name: field1, field2)]`.
    ///
    /// Each projection defines a subset of fields for a specific view.
    pub projections: Vec<ProjectionDef>,

    /// Whether soft delete is enabled.
    ///
    /// When `true`, the `delete` method sets `deleted_at` instead of removing
    /// the row, and all queries filter out records where `deleted_at IS NOT
    /// NULL`.
    pub soft_delete: bool,

    /// RETURNING clause mode for INSERT/UPDATE operations.
    ///
    /// Controls what data is fetched back from the database after writes.
    pub returning: ReturningMode
}

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

        let fields = match &input.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(named) => {
                    named.named.iter().map(FieldDef::from_field).collect()
                }
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
            has_many,
            projections,
            soft_delete: attrs.soft_delete,
            returning: attrs.returning
        })
    }

    /// Get the primary key field marked with `#[id]`.
    ///
    /// # Returns
    ///
    /// `Some(&FieldDef)` if an `#[id]` field exists, `None` otherwise.
    ///
    /// # Note
    ///
    /// Most generators require an id field. The SQL generator will
    /// panic if called without one. Consider validating this in
    /// `from_derive_input` for better error messages.
    pub fn id_field(&self) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.is_id())
    }

    /// Get fields to include in `CreateRequest` DTO.
    ///
    /// Returns fields where:
    /// - `#[field(create)]` is present
    /// - NOT marked with `#[id]` (IDs are auto-generated)
    /// - NOT marked with `#[auto]` (timestamps are auto-generated)
    /// - NOT marked with `#[field(skip)]`
    ///
    /// # Returns
    ///
    /// Vector of field references for the create DTO.
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
    ///
    /// # Returns
    ///
    /// Vector of field references for the update DTO.
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
    ///
    /// # Returns
    ///
    /// Vector of field references for the response DTO.
    pub fn response_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.in_response()).collect()
    }

    /// Get all fields for Row and Insertable structs.
    ///
    /// These database-layer structs include ALL fields from the
    /// entity, regardless of DTO inclusion settings.
    ///
    /// # Returns
    ///
    /// Slice of all field definitions.
    pub fn all_fields(&self) -> &[FieldDef] {
        &self.fields
    }

    /// Get fields with `#[belongs_to]` relations.
    ///
    /// Returns fields that are foreign keys to other entities.
    /// Used to generate relation methods in the repository.
    ///
    /// # Returns
    ///
    /// Vector of field references with belongs_to relations.
    pub fn relation_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.is_relation()).collect()
    }

    /// Get has-many relations defined via `#[has_many(Entity)]`.
    ///
    /// Returns entity identifiers for one-to-many relationships.
    /// Used to generate collection methods in the repository.
    ///
    /// # Returns
    ///
    /// Slice of related entity identifiers.
    pub fn has_many_relations(&self) -> &[Ident] {
        &self.has_many
    }

    /// Get the entity name as an identifier.
    ///
    /// # Returns
    ///
    /// Reference to the struct's `Ident`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let entity_name = entity.name(); // e.g., Ident("User")
    /// quote! { impl #entity_name { } }
    /// ```
    pub fn name(&self) -> &Ident {
        &self.ident
    }

    /// Get the entity name as a string.
    ///
    /// # Returns
    ///
    /// String representation of the entity name.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// entity.name_str() // "User"
    /// ```
    pub fn name_str(&self) -> String {
        self.ident.to_string()
    }

    /// Get the entity name in snake_case.
    ///
    /// Useful for generating function names, variable names, etc.
    ///
    /// # Returns
    ///
    /// Snake case version of the entity name.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// entity.snake_name() // "user", "user_profile", "order_item"
    /// ```
    #[allow(dead_code)]
    pub fn snake_name(&self) -> String {
        self.name_str().to_case(Case::Snake)
    }

    /// Get the fully qualified table name with schema.
    ///
    /// # Returns
    ///
    /// String in format `"schema.table"`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// entity.full_table_name() // "core.users", "public.products"
    /// ```
    pub fn full_table_name(&self) -> String {
        format!("{}.{}", self.schema, self.table)
    }

    /// Create a new identifier with prefix and/or suffix.
    ///
    /// Used to generate related type names following naming conventions.
    ///
    /// # Arguments
    ///
    /// * `prefix` - String to prepend (e.g., `"Create"`, `"Insertable"`)
    /// * `suffix` - String to append (e.g., `"Request"`, `"Row"`)
    ///
    /// # Returns
    ///
    /// New `Ident` at `call_site` span.
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
    ///
    /// # Returns
    ///
    /// Reference to the error type path.
    pub fn error_type(&self) -> &syn::Path {
        &self.error
    }

    /// Check if a custom error type is specified.
    ///
    /// Returns `true` if the error type is not the default `sqlx::Error`.
    ///
    /// # Returns
    ///
    /// `true` if custom error type is used.
    #[allow(dead_code)]
    pub fn has_custom_error(&self) -> bool {
        let default = default_error_type();
        self.error != default
    }

    /// Check if soft delete is enabled for this entity.
    ///
    /// # Returns
    ///
    /// `true` if `#[entity(soft_delete)]` is present.
    pub fn is_soft_delete(&self) -> bool {
        self.soft_delete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_error_type_is_sqlx_error() {
        let path = default_error_type();
        let path_str = quote::quote!(#path).to_string();
        assert!(path_str.contains("sqlx"));
        assert!(path_str.contains("Error"));
    }

    #[test]
    fn entity_def_error_type_accessor() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let error_path = entity.error_type();
        let path_str = quote::quote!(#error_path).to_string();
        assert!(path_str.contains("sqlx"));
    }

    #[test]
    fn entity_def_default_has_no_custom_error() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        assert!(!entity.has_custom_error());
    }

    #[test]
    fn entity_def_custom_error_detected() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users", error = "MyError")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        assert!(entity.has_custom_error());
    }
}
