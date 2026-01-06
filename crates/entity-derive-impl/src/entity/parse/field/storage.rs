// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Database storage configuration for entity fields.
//!
//! Controls database-specific behavior: primary keys, auto-generation,
//! and relations.
//!
//! # Relations
//!
//! Use `#[belongs_to(EntityName)]` on foreign key fields:
//!
//! ```rust,ignore
//! #[belongs_to(User)]
//! pub user_id: Uuid,
//! ```
//!
//! This generates a `find_user` method in the repository.

use syn::Ident;

/// Database storage configuration.
///
/// Determines how the field is stored and managed in the database.
///
/// # Attributes
///
/// - `#[id]` — Primary key with auto-generated UUID
/// - `#[auto]` — Auto-generated value (timestamps)
/// - `#[belongs_to(Entity)]` — Foreign key relation
///
/// # Future attributes (planned)
///
/// - `#[column(name = "...")]` — Custom column name
/// - `#[column(index)]` — Create index
/// - `#[column(unique)]` — Unique constraint
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
    pub is_auto: bool,

    /// Foreign key relation (`#[belongs_to(Entity)]`).
    ///
    /// Stores the related entity name. When set, generates:
    /// - `find_{entity}(&self, id) -> Result<Option<Entity>>` method
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[belongs_to(User)]
    /// pub user_id: Uuid,
    /// // Generates: async fn find_user(&self, post_id: Uuid) -> Result<Option<User>>
    /// ```
    pub belongs_to: Option<Ident>
}

impl StorageConfig {
    /// Check if this field is a foreign key relation.
    #[must_use]
    pub fn is_relation(&self) -> bool {
        self.belongs_to.is_some()
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;

    use super::*;

    #[test]
    fn default_is_not_special() {
        let config = StorageConfig::default();
        assert!(!config.is_id);
        assert!(!config.is_auto);
        assert!(!config.is_relation());
    }

    #[test]
    fn belongs_to_is_relation() {
        let config = StorageConfig {
            is_id:      false,
            is_auto:    false,
            belongs_to: Some(Ident::new("User", Span::call_site()))
        };
        assert!(config.is_relation());
    }
}
