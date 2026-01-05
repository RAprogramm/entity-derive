// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Field-level attribute parsing.
//!
//! This module handles parsing of field attributes and delegates to
//! specialized submodules for different concerns:
//!
//! - [`expose`] — DTO exposure (create, update, response, skip)
//! - [`storage`] — Database storage (id, auto, future: index, relation)
//!
//! # Architecture
//!
//! ```text
//! field.rs (coordinator)
//! ├── expose.rs   - DTO exposure configuration
//! └── storage.rs  - Database storage configuration
//! ```

mod expose;
mod storage;

pub use expose::ExposeConfig;
pub use storage::StorageConfig;
use syn::{Field, Ident, Type, Visibility};

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

    /// Field visibility.
    #[allow(dead_code)]
    pub vis: Visibility,

    /// DTO exposure configuration.
    pub expose: ExposeConfig,

    /// Database storage configuration.
    pub storage: StorageConfig
}

impl FieldDef {
    /// Parse field definition from syn's `Field`.
    ///
    /// Extracts base information and parses all attributes into
    /// exposure and storage configurations.
    pub fn from_field(field: &Field) -> Self {
        let ident = field.ident.clone().expect("named field required");
        let ty = field.ty.clone();
        let vis = field.vis.clone();

        let mut expose = ExposeConfig::default();
        let mut storage = StorageConfig::default();

        for attr in &field.attrs {
            if attr.path().is_ident("id") {
                storage.is_id = true;
            } else if attr.path().is_ident("auto") {
                storage.is_auto = true;
            } else if attr.path().is_ident("field") {
                expose = ExposeConfig::from_attr(attr);
            }
        }

        Self {
            ident,
            ty,
            vis,
            expose,
            storage
        }
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
}
