// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Database storage configuration for entity fields.
//!
//! Controls database-specific behavior: primary keys, auto-generation,
//! and future features like indexes and relations.

/// Database storage configuration.
///
/// Determines how the field is stored and managed in the database.
///
/// # Current attributes
///
/// - `#[id]` — Primary key with auto-generated UUID
/// - `#[auto]` — Auto-generated value (timestamps)
///
/// # Future attributes (planned)
///
/// - `#[column(name = "...")]` — Custom column name
/// - `#[column(index)]` — Create index
/// - `#[column(unique)]` — Unique constraint
/// - `#[column(relation = "...")]` — Foreign key relation
#[derive(Debug, Default, Clone)]
pub struct StorageConfig {
    /// Primary key field (`#[id]`).
    ///
    /// Effects:
    /// - Auto-generates UUID (v7 or v4 based on entity config)
    /// - Always included in Response DTO
    /// - Excluded from CreateRequest and UpdateRequest
    pub is_id: bool,

    /// Auto-generated field (`#[auto]`).
    ///
    /// Effects:
    /// - Gets `Default::default()` in From implementations
    /// - Excluded from CreateRequest and UpdateRequest
    /// - Typically used for `created_at`, `updated_at` timestamps
    pub is_auto: bool /* Future fields:
                       * pub column_name: Option<String>,
                       * pub index: bool,
                       * pub unique: bool,
                       * pub relation: Option<RelationConfig>, */
}

impl StorageConfig {
    /// Create config for ID field.
    #[must_use]
    #[allow(dead_code)]
    pub fn id() -> Self {
        Self {
            is_id:   true,
            is_auto: false
        }
    }

    /// Create config for auto-generated field.
    #[must_use]
    #[allow(dead_code)]
    pub fn auto() -> Self {
        Self {
            is_id:   false,
            is_auto: true
        }
    }

    /// Check if field value is auto-generated (ID or auto).
    #[must_use]
    #[allow(dead_code)]
    pub fn is_generated(&self) -> bool {
        self.is_id || self.is_auto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_special() {
        let config = StorageConfig::default();
        assert!(!config.is_id);
        assert!(!config.is_auto);
        assert!(!config.is_generated());
    }

    #[test]
    fn id_is_generated() {
        let config = StorageConfig::id();
        assert!(config.is_id);
        assert!(config.is_generated());
    }

    #[test]
    fn auto_is_generated() {
        let config = StorageConfig::auto();
        assert!(config.is_auto);
        assert!(config.is_generated());
    }
}
