// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Field-level attribute parsing.
//!
//! This module handles parsing of field attributes and delegates to
//! specialized submodules for different concerns:
//!
//! - [`expose`] — DTO exposure (create, update, response, skip)
//! - [`storage`] — Database storage (id, auto, belongs_to)
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

mod example;
mod expose;
mod filter;
mod storage;
mod validation;

pub use example::ExampleValue;
pub use expose::ExposeConfig;
pub use filter::{FilterConfig, FilterType};
pub use storage::StorageConfig;
use syn::{Attribute, Field, Ident, Type};
pub use validation::ValidationConfig;

use crate::utils::docs::extract_doc_comments;

/// Parse `#[belongs_to(EntityName)]` attribute.
///
/// Extracts the entity identifier from the attribute.
fn parse_belongs_to(attr: &Attribute) -> Option<Ident> {
    attr.parse_args::<Ident>().ok()
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
/// ```
#[derive(Debug)]
pub struct FieldDef {
    /// Field identifier (e.g., `id`, `name`, `created_at`).
    pub ident: Ident,

    /// Field type (e.g., `Uuid`, `Option<String>`, `DateTime<Utc>`).
    pub ty: Type,

    /// DTO exposure configuration.
    pub expose: ExposeConfig,

    /// Database storage configuration.
    pub storage: StorageConfig,

    /// Query filter configuration.
    pub filter: FilterConfig,

    /// Documentation comment from the field.
    ///
    /// Extracted from `///` comments for use in OpenAPI descriptions.
    #[allow(dead_code)] // Will be used for schema field descriptions (#78)
    pub doc: Option<String>,

    /// Validation configuration from `#[validate(...)]` attributes.
    ///
    /// Parsed for OpenAPI schema constraints and DTO validation.
    #[allow(dead_code)] // Will be used for OpenAPI schema constraints (#79)
    pub validation: ValidationConfig,

    /// Example value for OpenAPI schema.
    ///
    /// Parsed from `#[example = ...]` attribute.
    #[allow(dead_code)] // Will be used for OpenAPI schema examples (#80)
    pub example: Option<ExampleValue>
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

        let mut expose = ExposeConfig::default();
        let mut storage = StorageConfig::default();
        let mut filter = FilterConfig::default();

        for attr in &field.attrs {
            if attr.path().is_ident("id") {
                storage.is_id = true;
            } else if attr.path().is_ident("auto") {
                storage.is_auto = true;
            } else if attr.path().is_ident("field") {
                expose = ExposeConfig::from_attr(attr);
            } else if attr.path().is_ident("belongs_to") {
                storage.belongs_to = parse_belongs_to(attr);
            } else if attr.path().is_ident("filter") {
                filter = FilterConfig::from_attr(attr);
            }
        }

        Ok(Self {
            ident,
            ty,
            expose,
            storage,
            filter,
            doc,
            validation,
            example
        })
    }

    /// Get the field name as an identifier.
    #[must_use]
    pub fn name(&self) -> &Ident {
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
    pub fn ty(&self) -> &Type {
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
    pub fn is_id(&self) -> bool {
        self.storage.is_id
    }

    /// Check if this field is auto-generated.
    #[must_use]
    pub fn is_auto(&self) -> bool {
        self.storage.is_auto
    }

    /// Check if field should be in `CreateRequest`.
    #[must_use]
    pub fn in_create(&self) -> bool {
        self.expose.in_create()
    }

    /// Check if field should be in `UpdateRequest`.
    #[must_use]
    pub fn in_update(&self) -> bool {
        self.expose.in_update()
    }

    /// Check if field should be in `Response`.
    ///
    /// ID fields are always included regardless of expose config.
    #[must_use]
    pub fn in_response(&self) -> bool {
        !self.expose.skip && (self.expose.response || self.storage.is_id)
    }

    /// Get the related entity name if this is a foreign key.
    ///
    /// Returns `Some(Ident)` if `#[belongs_to(Entity)]` is present.
    #[must_use]
    pub fn belongs_to(&self) -> Option<&Ident> {
        self.storage.belongs_to.as_ref()
    }

    /// Check if this field is a foreign key relation.
    #[must_use]
    pub fn is_relation(&self) -> bool {
        self.storage.is_relation()
    }

    /// Check if this field has a filter configured.
    #[must_use]
    pub fn has_filter(&self) -> bool {
        self.filter.has_filter()
    }

    /// Get the filter configuration.
    #[must_use]
    pub fn filter(&self) -> &FilterConfig {
        &self.filter
    }

    /// Get the documentation comment if present.
    ///
    /// Returns the extracted doc comment for use in OpenAPI descriptions.
    #[must_use]
    #[allow(dead_code)] // Will be used for schema field descriptions (#78)
    pub fn doc(&self) -> Option<&str> {
        self.doc.as_deref()
    }

    /// Get the validation configuration.
    ///
    /// Returns the parsed validation rules for OpenAPI constraints.
    #[must_use]
    #[allow(dead_code)] // Will be used for OpenAPI schema constraints (#79)
    pub fn validation(&self) -> &ValidationConfig {
        &self.validation
    }

    /// Check if this field has validation rules.
    #[must_use]
    #[allow(dead_code)] // Will be used for OpenAPI schema constraints (#79)
    pub fn has_validation(&self) -> bool {
        self.validation.has_validation()
    }

    /// Get the example value if present.
    ///
    /// Returns the parsed example for use in OpenAPI schema.
    #[must_use]
    #[allow(dead_code)] // Will be used for OpenAPI schema examples (#80)
    pub fn example(&self) -> Option<&ExampleValue> {
        self.example.as_ref()
    }

    /// Check if this field has an example value.
    #[must_use]
    #[allow(dead_code)] // Will be used for OpenAPI schema examples (#80)
    pub fn has_example(&self) -> bool {
        self.example.is_some()
    }
}
