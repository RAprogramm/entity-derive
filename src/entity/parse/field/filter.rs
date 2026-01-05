// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Filter configuration for query generation.
//!
//! This module defines filter types that can be applied to entity fields
//! for type-safe query generation.

use syn::Attribute;

/// Filter type for a field.
///
/// Determines what kind of SQL condition is generated for this field
/// in the query struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    /// No filter for this field.
    #[default]
    None,

    /// Exact match filter.
    ///
    /// Generates: `WHERE field = $n`
    Eq,

    /// LIKE pattern match filter.
    ///
    /// Generates: `WHERE field LIKE $n`
    /// The value should include wildcards (e.g., `%search%`).
    Like,

    /// Range filter for comparable types.
    ///
    /// Generates two optional fields:
    /// - `field_from`: `WHERE field >= $n`
    /// - `field_to`: `WHERE field <= $n`
    Range
}

/// Filter configuration for a field.
///
/// Parsed from `#[filter]`, `#[filter(like)]`, or `#[filter(range)]`.
#[derive(Debug, Clone, Default)]
pub struct FilterConfig {
    /// The type of filter to apply.
    pub filter_type: FilterType
}

impl FilterConfig {
    /// Parse filter configuration from `#[filter(...)]` attribute.
    ///
    /// # Syntax
    ///
    /// - `#[filter]` — exact match (default)
    /// - `#[filter(eq)]` — exact match (explicit)
    /// - `#[filter(like)]` — LIKE pattern match
    /// - `#[filter(range)]` — range filter (from/to)
    pub fn from_attr(attr: &Attribute) -> Self {
        let filter_type = attr
            .parse_args_with(|input: syn::parse::ParseStream<'_>| {
                let ident: syn::Ident = input.parse()?;
                Ok(ident.to_string())
            })
            .ok()
            .map(|s| match s.as_str() {
                "eq" => FilterType::Eq,
                "like" => FilterType::Like,
                "range" => FilterType::Range,
                _ => FilterType::Eq
            })
            .unwrap_or(FilterType::Eq);

        Self {
            filter_type
        }
    }

    /// Check if this field has any filter.
    pub fn has_filter(&self) -> bool {
        self.filter_type != FilterType::None
    }

    /// Check if this is an exact match filter.
    #[allow(dead_code)]
    pub fn is_eq(&self) -> bool {
        self.filter_type == FilterType::Eq
    }

    /// Check if this is a LIKE filter.
    #[allow(dead_code)]
    pub fn is_like(&self) -> bool {
        self.filter_type == FilterType::Like
    }

    /// Check if this is a range filter.
    #[allow(dead_code)]
    pub fn is_range(&self) -> bool {
        self.filter_type == FilterType::Range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_none() {
        let config = FilterConfig::default();
        assert_eq!(config.filter_type, FilterType::None);
        assert!(!config.has_filter());
    }
}
