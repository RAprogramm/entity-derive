// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! DTO exposure configuration for entity fields.
//!
//! Controls which DTOs a field appears in: CreateRequest, UpdateRequest,
//! Response.

use syn::{Attribute, Meta};

/// DTO exposure configuration.
///
/// Determines which generated DTOs will include this field.
///
/// # Examples
///
/// ```rust,ignore
/// #[field(create, update, response)]  // All DTOs
/// #[field(create, response)]          // Not in UpdateRequest
/// #[field(skip)]                      // Excluded from all
/// ```
#[derive(Debug, Default, Clone)]
pub struct ExposeConfig {
    /// Include in `CreateRequest` DTO.
    pub create: bool,

    /// Include in `UpdateRequest` DTO.
    ///
    /// Fields are automatically wrapped in `Option<T>` for partial updates.
    pub update: bool,

    /// Include in `Response` DTO.
    pub response: bool,

    /// Exclude from all DTOs.
    ///
    /// Overrides all other flags. Use for sensitive data like passwords.
    pub skip: bool
}

impl ExposeConfig {
    /// Parse exposure config from `#[field(...)]` attribute.
    ///
    /// # Recognized options
    ///
    /// - `create` → include in CreateRequest
    /// - `update` → include in UpdateRequest
    /// - `response` → include in Response
    /// - `skip` → exclude from all DTOs
    pub fn from_attr(attr: &Attribute) -> Self {
        let mut config = Self::default();

        if let Meta::List(meta_list) = &attr.meta {
            let _ = meta_list.parse_nested_meta(|meta| {
                if meta.path.is_ident("create") {
                    config.create = true;
                } else if meta.path.is_ident("update") {
                    config.update = true;
                } else if meta.path.is_ident("response") {
                    config.response = true;
                } else if meta.path.is_ident("skip") {
                    config.skip = true;
                }
                Ok(())
            });
        }

        config
    }

    /// Check if field should appear in CreateRequest.
    #[must_use]
    pub fn in_create(&self) -> bool {
        !self.skip && self.create
    }

    /// Check if field should appear in UpdateRequest.
    #[must_use]
    pub fn in_update(&self) -> bool {
        !self.skip && self.update
    }

    /// Check if field should appear in Response.
    ///
    /// Note: ID fields are always included regardless of this flag.
    #[must_use]
    #[allow(dead_code)]
    pub fn in_response(&self) -> bool {
        !self.skip && self.response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_false() {
        let config = ExposeConfig::default();
        assert!(!config.create);
        assert!(!config.update);
        assert!(!config.response);
        assert!(!config.skip);
    }

    #[test]
    fn skip_overrides_all() {
        let config = ExposeConfig {
            create:   true,
            update:   true,
            response: true,
            skip:     true
        };
        assert!(!config.in_create());
        assert!(!config.in_update());
        assert!(!config.in_response());
    }
}
