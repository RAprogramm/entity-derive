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
//!
//! ## With ON DELETE action
//!
//! ```rust,ignore
//! #[belongs_to(User, on_delete = "cascade")]
//! pub user_id: Uuid,
//! ```

use syn::Ident;

use super::ReferentialAction;

/// Database storage configuration.
///
/// Determines how the field is stored and managed in the database.
///
/// # Attributes
///
/// - `#[id]` — Primary key with auto-generated UUID
/// - `#[auto]` — Auto-generated value (timestamps)
/// - `#[belongs_to(Entity)]` — Foreign key relation
/// - `#[belongs_to(Entity, on_delete = "cascade")]` — FK with ON DELETE
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
    /// - REFERENCES clause in migration (if migrations enabled)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[belongs_to(User)]
    /// pub user_id: Uuid,
    /// // Generates: async fn find_user(&self, post_id: Uuid) -> Result<Option<User>>
    /// ```
    pub belongs_to: Option<Ident>,

    /// ON DELETE action for foreign key.
    ///
    /// Only applies when `belongs_to` is set.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[belongs_to(User, on_delete = "cascade")]
    /// pub user_id: Uuid,
    /// // Generates: REFERENCES users(id) ON DELETE CASCADE
    /// ```
    pub on_delete: Option<ReferentialAction>
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
        assert!(config.on_delete.is_none());
    }

    #[test]
    fn belongs_to_is_relation() {
        let config = StorageConfig {
            is_id:      false,
            is_auto:    false,
            belongs_to: Some(Ident::new("User", Span::call_site())),
            on_delete:  None
        };
        assert!(config.is_relation());
    }

    #[test]
    fn belongs_to_with_on_delete() {
        let config = StorageConfig {
            is_id:      false,
            is_auto:    false,
            belongs_to: Some(Ident::new("User", Span::call_site())),
            on_delete:  Some(ReferentialAction::Cascade)
        };
        assert!(config.is_relation());
        assert_eq!(config.on_delete, Some(ReferentialAction::Cascade));
    }
}
