// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Field-level attribute parsing.
//!
//! This module handles parsing of field attributes and delegates to
//! specialized submodules for different concerns:
//!
//! - [`expose`] — DTO exposure (create, update, response, skip)
//! - [`storage`] — Database storage (id, auto, `belongs_to`)
//!
//! # Architecture
//!
//! ```text
//! field.rs (coordinator)
//! ├── expose.rs   - DTO exposure configuration
//! └── storage.rs  - Database storage configuration
//! ```
//!
//! # Relations
//!
//! Foreign key relations are declared with `#[belongs_to(Entity)]`:
//!
//! ```rust,ignore
//! #[belongs_to(User)]
//! pub user_id: Uuid,
//! ```

mod column;
mod example;
mod expose;
mod filter;
pub mod map;
mod schema;
mod storage;
mod validation;

pub use column::{ColumnConfig, IndexType, ReferentialAction};
pub use example::ExampleValue;
pub use expose::ExposeConfig;
pub use filter::{FilterConfig, FilterType};
pub use map::MapConfig;
pub use storage::StorageConfig;
use syn::{Attribute, Field, Ident, Type};
pub use validation::ValidationConfig;

use crate::utils::docs::extract_doc_comments;

/// Parse `#[belongs_to(EntityName)]` or `#[belongs_to(EntityName, on_delete =
/// "cascade")]`.
///
/// Returns the entity identifier and optional ON DELETE action.
fn parse_belongs_to(attr: &Attribute) -> (Option<Ident>, Option<ReferentialAction>) {
    // Try simple case: #[belongs_to(Entity)]
    if let Ok(ident) = attr.parse_args::<Ident>() {
        return (Some(ident), None);
    }

    // Try extended case: #[belongs_to(Entity, on_delete = "cascade")]
    let mut entity = None;
    let mut on_delete = None;

    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("on_delete") {
            let _: syn::Token![=] = meta.input.parse()?;
            let value: syn::LitStr = meta.input.parse()?;
            on_delete = ReferentialAction::from_str(&value.value());
        } else if let Some(ident) = meta.path.get_ident() {
            entity = Some(ident.clone());
        }
        Ok(())
    });

    (entity, on_delete)
}

/// Field definition with all parsed attributes.
///
/// Represents a single field from the entity struct, combining
/// base field information with exposure and storage configurations.
///
/// # Example
///
/// ```rust,ignore
/// #[id]                              // StorageConfig::is_id = true
/// pub id: Uuid,
///
/// #[field(create, update, response)] // ExposeConfig
/// pub name: String,
///
/// #[auto]                            // StorageConfig::is_auto = true
/// #[field(response)]
/// pub created_at: DateTime<Utc>,
///
/// #[column(unique, index)]           // ColumnConfig
/// pub email: String,
/// ```
/// Embedded value-object declaration parsed from
/// `#[embed(prefix = "price_", fields(amount_cents: i64, currency: String))]`.
#[derive(Debug, Clone)]
pub struct EmbedConfig {
    /// Column-name prefix for the flattened subfields.
    pub prefix: String,

    /// Declared subfields of the embedded struct.
    pub subfields: Vec<(Ident, Type)>
}

impl EmbedConfig {
    /// Parse an `#[embed(...)]` attribute.
    ///
    /// # Errors
    ///
    /// Returns an error when `fields(...)` is missing/empty, an option
    /// is unknown, or the syntax does not match `name: Type` pairs.
    pub fn from_attr(attr: &syn::Attribute) -> darling::Result<Self> {
        let mut prefix: Option<String> = None;
        let mut subfields: Vec<(Ident, Type)> = Vec::new();

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("prefix") {
                let value: syn::LitStr = meta.value()?.parse()?;
                prefix = Some(value.value());
            } else if meta.path.is_ident("fields") {
                let content;
                syn::parenthesized!(content in meta.input);
                while !content.is_empty() {
                    let name: Ident = content.parse()?;
                    let _: syn::Token![:] = content.parse()?;
                    let ty: Type = content.parse()?;
                    subfields.push((name, ty));
                    if content.peek(syn::Token![,]) {
                        let _: syn::Token![,] = content.parse()?;
                    }
                }
            } else {
                return Err(meta.error(
                    "unknown embed option; expected prefix = \"...\" and fields(name: Type, ...)"
                ));
            }
            Ok(())
        })
        .map_err(darling::Error::from)?;

        if subfields.is_empty() {
            return Err(darling::Error::custom(
                "embed requires fields(name: Type, ...) with at least one subfield"
            )
            .with_span(attr));
        }

        Ok(Self {
            prefix: prefix.unwrap_or_default(),
            subfields
        })
    }
}

#[derive(Debug)]
pub struct FieldDef {
    /// Field identifier (e.g., `id`, `name`, `created_at`).
    pub ident: Ident,

    /// Field type (e.g., `Uuid`, `Option<String>`, `DateTime<Utc>`).
    pub ty: Type,

    /// Whether the field is marked `#[sort]` (dynamic ORDER BY).
    pub sortable: bool,

    /// Embedded value-object configuration from `#[embed(...)]`.
    ///
    /// `Some` marks an embed parent: the field lives on the entity as
    /// a struct but maps to several prefixed columns.
    pub embed: Option<EmbedConfig>,

    /// Synthetic embed column origin.
    ///
    /// `Some((parent, subfield))` marks a generated column field that
    /// flattens `parent.subfield`. Such fields never appear in DTOs.
    pub embed_origin: Option<(Ident, Ident)>,

    /// DTO exposure configuration.
    pub expose: ExposeConfig,

    /// Database storage configuration.
    pub storage: StorageConfig,

    /// Query filter configuration.
    pub filter: FilterConfig,

    /// Column configuration for migrations.
    ///
    /// Parsed from `#[column(...)]` attributes for constraints and indexes.
    pub column: ColumnConfig,

    /// Documentation comment from the field.
    ///
    /// Extracted from `///` comments for use in `OpenAPI` descriptions.
    #[allow(dead_code)] // Will be used for schema field descriptions (#78)
    pub doc: Option<String>,

    /// Validation configuration from `#[validate(...)]` attributes.
    ///
    /// Parsed for `OpenAPI` schema constraints and DTO validation.
    #[allow(dead_code)] // Will be used for OpenAPI schema constraints (#79)
    pub validation: ValidationConfig,

    /// Example value for `OpenAPI` schema.
    ///
    /// Parsed from `#[example = ...]` attribute.
    #[allow(dead_code)] // Will be used for OpenAPI schema examples (#80)
    pub example: Option<ExampleValue>,

    /// Row-to-entity mapping configuration.
    ///
    /// Parsed from `#[map(...)]` attributes for transforming fields
    /// in the `From<Row> for Entity` implementation.
    pub map: MapConfig,

    /// `OpenAPI` schema override carried from `#[schema(...)]`.
    ///
    /// The token list utoipa is given verbatim on every generated
    /// struct that derives `utoipa::ToSchema`.
    pub schema: Option<proc_macro2::TokenStream>
}

impl FieldDef {
    /// Parse field definition from syn's `Field`.
    ///
    /// Extracts base information and parses all attributes into
    /// exposure and storage configurations.
    ///
    /// # Errors
    ///
    /// Returns error if the field has no identifier (tuple struct field).
    pub fn from_field(field: &Field) -> darling::Result<Self> {
        let ident = field.ident.clone().ok_or_else(|| {
            darling::Error::custom("Entity fields must be named").with_span(field)
        })?;
        let ty = field.ty.clone();
        let doc = extract_doc_comments(&field.attrs);
        let validation = validation::parse_validation_attrs(&field.attrs);
        let example = example::parse_example_attr(&field.attrs);
        let schema = schema::parse_schema_attr(&field.attrs);

        let mut sortable = false;
        let mut embed: Option<EmbedConfig> = None;
        let mut expose = ExposeConfig::default();
        let mut storage = StorageConfig::default();
        let mut filter = FilterConfig::default();
        let mut column = ColumnConfig::default();
        let mut map = MapConfig::default();

        for attr in &field.attrs {
            if attr.path().is_ident("id") {
                storage.is_id = true;
            } else if attr.path().is_ident("auto") {
                storage.is_auto = true;
            } else if attr.path().is_ident("owner") {
                storage.is_owner = true;
            } else if attr.path().is_ident("sort") {
                sortable = true;
            } else if attr.path().is_ident("version") {
                storage.is_version = true;
            } else if attr.path().is_ident("embed") {
                embed = Some(EmbedConfig::from_attr(attr)?);
            } else if attr.path().is_ident("field") {
                expose = ExposeConfig::from_attr(attr);
            } else if attr.path().is_ident("belongs_to") {
                let (entity, on_del) = parse_belongs_to(attr);
                storage.belongs_to = entity;
                storage.on_delete = on_del;
            } else if attr.path().is_ident("filter") {
                filter = FilterConfig::from_attr(attr);
            } else if attr.path().is_ident("column") {
                column = ColumnConfig::from_attr(attr);
            } else if attr.path().is_ident("map")
                && let Some(parsed) = MapConfig::from_attr(attr)
            {
                map = parsed;
            }
        }

        Ok(Self {
            ident,
            ty,
            sortable,
            embed,
            embed_origin: None,
            expose,
            storage,
            filter,
            column,
            doc,
            validation,
            example,
            map,
            schema
        })
    }

    /// The `#[schema(...)]` attribute to place on a generated field.
    ///
    /// Empty unless the entity declared one and the `api` feature is on:
    /// with it off nothing generated derives `utoipa::ToSchema`, and the
    /// attribute would land on a struct that cannot interpret it.
    #[must_use]
    pub fn schema_attr(&self) -> proc_macro2::TokenStream {
        match &self.schema {
            Some(tokens) if cfg!(feature = "api") => quote::quote! { #[schema(#tokens)] },
            _ => proc_macro2::TokenStream::new()
        }
    }

    /// Get the field name as an identifier.
    #[must_use]
    pub const fn name(&self) -> &Ident {
        &self.ident
    }

    /// Get the field name as a string.
    ///
    /// Used for generating SQL column names.
    #[must_use]
    pub fn name_str(&self) -> String {
        self.ident.to_string()
    }

    /// Get the field type.
    #[must_use]
    pub const fn ty(&self) -> &Type {
        &self.ty
    }

    /// Inner type of `Option<T>`, or the type itself when not optional.
    ///
    /// Case-insensitive lookups take the unwrapped value: a NULL column
    /// never matches a `LOWER(...)` probe, so an `Option` parameter adds
    /// nothing but ceremony.
    #[must_use]
    pub fn option_inner_type(&self) -> &Type {
        if let Type::Path(type_path) = &self.ty
            && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Option"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            return inner;
        }
        &self.ty
    }

    /// Check if the field type is `Option<T>`.
    ///
    /// Used to determine whether to wrap update fields in `Option`.
    #[must_use]
    pub fn is_option(&self) -> bool {
        if let Type::Path(type_path) = &self.ty
            && let Some(segment) = type_path.path.segments.last()
        {
            return segment.ident == "Option";
        }
        false
    }

    /// Check if this is the primary key field.
    #[must_use]
    pub const fn is_id(&self) -> bool {
        self.storage.is_id
    }

    /// Check if this field is auto-generated.
    #[must_use]
    pub const fn is_auto(&self) -> bool {
        self.storage.is_auto
    }

    /// Check if field should be in `CreateRequest`.
    #[must_use]
    pub const fn in_create(&self) -> bool {
        self.expose.in_create()
    }

    /// Check if field should be in `UpdateRequest`.
    #[must_use]
    pub const fn in_update(&self) -> bool {
        self.expose.in_update()
    }

    /// Check if field should be in `Response`.
    ///
    /// ID fields are always included regardless of expose config.
    #[must_use]
    pub const fn in_response(&self) -> bool {
        !self.expose.skip && (self.expose.response || self.storage.is_id)
    }

    /// Get the related entity name if this is a foreign key.
    ///
    /// Returns `Some(Ident)` if `#[belongs_to(Entity)]` is present.
    #[must_use]
    pub const fn belongs_to(&self) -> Option<&Ident> {
        self.storage.belongs_to.as_ref()
    }

    /// Check if this field is a foreign key relation.
    #[must_use]
    pub const fn is_relation(&self) -> bool {
        self.storage.is_relation()
    }

    /// Check if this field has a filter configured.
    #[must_use]
    pub fn has_filter(&self) -> bool {
        self.filter.has_filter()
    }

    /// Get the filter configuration.
    #[must_use]
    pub const fn filter(&self) -> &FilterConfig {
        &self.filter
    }

    /// Get the documentation comment if present.
    ///
    /// Returns the extracted doc comment for use in `OpenAPI` descriptions.
    #[must_use]
    #[allow(dead_code)] // Will be used for schema field descriptions (#78)
    pub fn doc(&self) -> Option<&str> {
        self.doc.as_deref()
    }

    /// Get the validation configuration.
    ///
    /// Returns the parsed validation rules for `OpenAPI` constraints.
    #[must_use]
    #[allow(dead_code)] // Will be used for OpenAPI schema constraints (#79)
    pub const fn validation(&self) -> &ValidationConfig {
        &self.validation
    }

    /// Check if this field has validation rules.
    #[must_use]
    #[allow(dead_code)] // Will be used for OpenAPI schema constraints (#79)
    pub const fn has_validation(&self) -> bool {
        self.validation.has_validation()
    }

    /// Get the example value if present.
    ///
    /// Returns the parsed example for use in `OpenAPI` schema.
    #[must_use]
    #[allow(dead_code)] // Will be used for OpenAPI schema examples (#80)
    pub const fn example(&self) -> Option<&ExampleValue> {
        self.example.as_ref()
    }

    /// Check if this field has an example value.
    #[must_use]
    #[allow(dead_code)] // Will be used for OpenAPI schema examples (#80)
    pub const fn has_example(&self) -> bool {
        self.example.is_some()
    }

    /// Get the column configuration.
    ///
    /// Returns parsed column constraints and index settings.
    #[must_use]
    pub const fn column(&self) -> &ColumnConfig {
        &self.column
    }

    /// Get the row-to-entity mapping configuration.
    ///
    /// Returns the parsed mapping rules for the `From<Row> for Entity`
    /// implementation.
    #[must_use]
    pub const fn map(&self) -> &MapConfig {
        &self.map
    }

    /// Check if this field has a mapping configuration.
    #[must_use]
    #[allow(dead_code)] // Public API for future use
    pub const fn has_map(&self) -> bool {
        !matches!(self.map, MapConfig::None)
    }

    /// Check if this column has a UNIQUE constraint.
    #[must_use]
    pub const fn is_unique(&self) -> bool {
        self.column.unique
    }

    /// Check if this column should be indexed.
    #[must_use]
    #[allow(dead_code)] // Public API for future use
    pub const fn has_index(&self) -> bool {
        self.column.has_index()
    }

    /// Get the database column name.
    ///
    /// Returns custom name if set, otherwise the field name.
    #[must_use]
    pub fn column_name(&self) -> String {
        self.column.column_name(&self.name_str()).to_string()
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn parse_field(tokens: proc_macro2::TokenStream) -> FieldDef {
        let field: Field = parse_quote!(#tokens);
        FieldDef::from_field(&field).unwrap()
    }

    #[test]
    fn field_basic_parsing() {
        let field = parse_field(quote::quote! { pub name: String });
        assert_eq!(field.name_str(), "name");
        assert!(!field.is_id());
        assert!(!field.is_auto());
    }

    #[test]
    fn field_id_attribute() {
        let field = parse_field(quote::quote! {
            #[id]
            pub id: uuid::Uuid
        });
        assert!(field.is_id());
        assert!(field.in_response());
    }

    #[test]
    fn field_auto_attribute() {
        let field = parse_field(quote::quote! {
            #[auto]
            pub created_at: chrono::DateTime<chrono::Utc>
        });
        assert!(field.is_auto());
    }

    #[test]
    fn field_expose_config() {
        let field = parse_field(quote::quote! {
            #[field(create, update, response)]
            pub name: String
        });
        assert!(field.in_create());
        assert!(field.in_update());
        assert!(field.in_response());
    }

    #[test]
    fn field_expose_skip() {
        let field = parse_field(quote::quote! {
            #[field(skip)]
            pub password: String
        });
        assert!(!field.in_create());
        assert!(!field.in_update());
        assert!(!field.in_response());
    }

    #[test]
    fn field_belongs_to() {
        let field = parse_field(quote::quote! {
            #[belongs_to(User)]
            pub user_id: uuid::Uuid
        });
        assert!(field.is_relation());
        assert!(field.belongs_to().is_some());
        assert_eq!(field.belongs_to().unwrap().to_string(), "User");
        assert!(field.storage.on_delete.is_none());
    }

    #[test]
    fn field_belongs_to_with_on_delete() {
        let field = parse_field(quote::quote! {
            #[belongs_to(User, on_delete = "cascade")]
            pub user_id: uuid::Uuid
        });
        assert!(field.is_relation());
        assert_eq!(field.belongs_to().unwrap().to_string(), "User");
        assert_eq!(field.storage.on_delete, Some(ReferentialAction::Cascade));
    }

    #[test]
    fn field_belongs_to_with_on_delete_set_null() {
        let field = parse_field(quote::quote! {
            #[belongs_to(Organization, on_delete = "set null")]
            pub org_id: uuid::Uuid
        });
        assert!(field.is_relation());
        assert_eq!(field.belongs_to().unwrap().to_string(), "Organization");
        assert_eq!(field.storage.on_delete, Some(ReferentialAction::SetNull));
    }

    #[test]
    fn field_filter_attribute() {
        let field = parse_field(quote::quote! {
            #[filter]
            pub status: String
        });
        assert!(field.has_filter());
    }

    #[test]
    fn field_is_option() {
        let field = parse_field(quote::quote! { pub avatar: Option<String> });
        assert!(field.is_option());

        let field2 = parse_field(quote::quote! { pub name: String });
        assert!(!field2.is_option());
    }

    #[test]
    fn field_ty_accessor() {
        let field = parse_field(quote::quote! { pub count: i32 });
        let ty = field.ty();
        let ty_str = quote::quote!(#ty).to_string();
        assert!(ty_str.contains("i32"));
    }

    #[test]
    fn field_doc_comment() {
        let field = parse_field(quote::quote! {
            /// User's display name
            pub name: String
        });
        assert!(field.doc().is_some());
        assert!(field.doc().unwrap().contains("display name"));
    }

    #[test]
    fn field_no_doc_comment() {
        let field = parse_field(quote::quote! { pub name: String });
        assert!(field.doc().is_none());
    }

    #[test]
    fn field_validation_accessor() {
        let field = parse_field(quote::quote! { pub name: String });
        let _validation = field.validation();
        assert!(!field.has_validation());
    }

    #[test]
    fn field_example_accessor() {
        let field = parse_field(quote::quote! { pub name: String });
        assert!(field.example().is_none());
        assert!(!field.has_example());
    }

    #[test]
    fn field_filter_accessor() {
        let field = parse_field(quote::quote! {
            #[filter(like)]
            pub name: String
        });
        let filter = field.filter();
        assert!(filter.has_filter());
    }

    #[test]
    fn field_name_accessor() {
        let field = parse_field(quote::quote! { pub email: String });
        assert_eq!(field.name().to_string(), "email");
    }

    #[test]
    fn field_column_unique() {
        let field = parse_field(quote::quote! {
            #[column(unique)]
            pub email: String
        });
        assert!(field.is_unique());
    }

    #[test]
    fn field_column_index() {
        let field = parse_field(quote::quote! {
            #[column(index)]
            pub status: String
        });
        assert!(field.has_index());
        assert_eq!(field.column().index, Some(IndexType::BTree));
    }

    #[test]
    fn field_column_index_gin() {
        let field = parse_field(quote::quote! {
            #[column(index = "gin")]
            pub tags: Vec<String>
        });
        assert!(field.has_index());
        assert_eq!(field.column().index, Some(IndexType::Gin));
    }

    #[test]
    fn field_column_default() {
        let field = parse_field(quote::quote! {
            #[column(default = "true")]
            pub is_active: bool
        });
        assert_eq!(field.column().default, Some("true".to_string()));
    }

    #[test]
    fn field_column_check() {
        let field = parse_field(quote::quote! {
            #[column(check = "age >= 0")]
            pub age: i32
        });
        assert_eq!(field.column().check, Some("age >= 0".to_string()));
    }

    #[test]
    fn field_column_varchar() {
        let field = parse_field(quote::quote! {
            #[column(varchar = 100)]
            pub name: String
        });
        assert_eq!(field.column().varchar, Some(100));
    }

    #[test]
    fn field_column_custom_name() {
        let field = parse_field(quote::quote! {
            #[column(name = "user_email")]
            pub email: String
        });
        assert_eq!(field.column_name(), "user_email");
    }

    #[test]
    fn field_column_default_name() {
        let field = parse_field(quote::quote! { pub email: String });
        assert_eq!(field.column_name(), "email");
    }

    #[test]
    fn field_column_multiple_attrs() {
        let field = parse_field(quote::quote! {
            #[column(unique, index, default = "NOW()")]
            pub created_at: DateTime<Utc>
        });
        assert!(field.is_unique());
        assert!(field.has_index());
        assert_eq!(field.column().default, Some("NOW()".to_string()));
    }

    #[test]
    fn field_map_empty_to_none() {
        let field = parse_field(quote::quote! {
            #[map(empty_to_none)]
            pub nickname: Option<String>
        });
        assert!(matches!(field.map(), MapConfig::EmptyToNone));
        assert!(field.has_map());
    }

    #[test]
    fn field_map_unwrap_default() {
        let field = parse_field(quote::quote! {
            #[map(unwrap_default)]
            pub age: Option<i32>
        });
        assert!(matches!(field.map(), MapConfig::UnwrapDefault));
        assert!(field.has_map());
    }

    #[test]
    fn field_map_now() {
        let field = parse_field(quote::quote! {
            #[map(now)]
            pub last_seen: Option<chrono::DateTime<chrono::Utc>>
        });
        assert!(matches!(field.map(), MapConfig::Now));
        assert!(field.has_map());
    }

    #[test]
    fn field_map_expr() {
        let field = parse_field(quote::quote! {
            #[map(expr = "row.raw.parse().unwrap_or(0)")]
            pub score: i32
        });
        assert!(matches!(field.map(), MapConfig::Expr(s) if s == "row.raw.parse().unwrap_or(0)"));
        assert!(field.has_map());
    }

    #[test]
    fn field_no_map() {
        let field = parse_field(quote::quote! { pub name: String });
        assert!(matches!(field.map(), MapConfig::None));
        assert!(!field.has_map());
    }

    #[test]
    fn field_map_with_other_attrs() {
        let field = parse_field(quote::quote! {
            #[field(create, response)]
            #[map(empty_to_none)]
            pub nickname: Option<String>
        });
        assert!(field.in_create());
        assert!(field.in_response());
        assert!(matches!(field.map(), MapConfig::EmptyToNone));
        assert!(field.has_map());
    }
}
