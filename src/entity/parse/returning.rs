// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! RETURNING clause configuration.
//!
//! This module defines [`ReturningMode`], which controls what data is returned
//! from INSERT and UPDATE operations.

use darling::FromMeta;

/// RETURNING mode for INSERT/UPDATE operations.
///
/// Controls what data is fetched back from the database after write operations.
/// This affects performance and determines what fields are available in the
/// returned entity.
///
/// # Variants
///
/// | Mode | RETURNING Clause | Use Case |
/// |------|-----------------|----------|
/// | `Full` | `RETURNING *` | Need all fields including DB-generated |
/// | `Id` | `RETURNING id` | Only need to confirm the ID |
/// | `None` | (no RETURNING) | Fire-and-forget, return pre-built entity |
///
/// # Examples
///
/// ```rust,ignore
/// // Full - get all fields back from DB (default)
/// #[entity(table = "users", returning = "full")]
///
/// // ID only - just confirm the insert
/// #[entity(table = "users", returning = "id")]
///
/// // None - don't fetch anything back (fastest)
/// #[entity(table = "users", returning = "none")]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ReturningMode {
    /// Return all fields from the database.
    ///
    /// Uses `RETURNING *` to fetch the complete row including any
    /// database-generated values (sequences, triggers, defaults).
    /// This is the safest option when the DB might modify data.
    #[default]
    Full,

    /// Return only the primary key.
    ///
    /// Uses `RETURNING id` to get just the ID back. Useful when you
    /// only need to confirm the insert succeeded and get the ID.
    Id,

    /// Don't use RETURNING clause.
    ///
    /// Returns the pre-constructed entity without fetching from DB.
    /// Fastest option, but won't reflect any database-side modifications
    /// (triggers, default values, etc.).
    None,

    /// Return specific columns.
    ///
    /// Uses `RETURNING col1, col2, ...` to fetch only specified fields.
    /// The entity is constructed from DTO first, then updated with
    /// returned values from the database.
    Custom(Vec<String>)
}

impl FromMeta for ReturningMode {
    /// Parse returning mode from string attribute value.
    ///
    /// # Accepted Values
    ///
    /// - `"full"` → [`ReturningMode::Full`]
    /// - `"id"` → [`ReturningMode::Id`]
    /// - `"none"` → [`ReturningMode::None`]
    /// - `"col1, col2, col3"` → [`ReturningMode::Custom`] with specified
    ///   columns
    ///
    /// Values are case-insensitive for keywords.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", returning = "full")]
    /// #[entity(table = "users", returning = "id, created_at")]
    /// #[entity(table = "users", returning = "id, name, updated_at")]
    /// ```
    fn from_string(value: &str) -> darling::Result<Self> {
        match value.to_lowercase().as_str() {
            "full" => Ok(ReturningMode::Full),
            "id" => Ok(ReturningMode::Id),
            "none" => Ok(ReturningMode::None),
            _ => {
                // Parse as comma-separated column list
                let columns: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if columns.is_empty() {
                    Err(darling::Error::unknown_value(value))
                } else {
                    Ok(ReturningMode::Custom(columns))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_full() {
        assert_eq!(ReturningMode::default(), ReturningMode::Full);
    }

    #[test]
    fn from_meta_keywords() {
        assert_eq!(
            ReturningMode::from_string("full").unwrap(),
            ReturningMode::Full
        );
        assert_eq!(
            ReturningMode::from_string("FULL").unwrap(),
            ReturningMode::Full
        );
        assert_eq!(ReturningMode::from_string("id").unwrap(), ReturningMode::Id);
        assert_eq!(ReturningMode::from_string("ID").unwrap(), ReturningMode::Id);
        assert_eq!(
            ReturningMode::from_string("none").unwrap(),
            ReturningMode::None
        );
        assert_eq!(
            ReturningMode::from_string("NONE").unwrap(),
            ReturningMode::None
        );
    }

    #[test]
    fn from_meta_custom_columns() {
        assert_eq!(
            ReturningMode::from_string("id, created_at").unwrap(),
            ReturningMode::Custom(vec!["id".to_string(), "created_at".to_string()])
        );
        assert_eq!(
            ReturningMode::from_string("id,name,updated_at").unwrap(),
            ReturningMode::Custom(vec![
                "id".to_string(),
                "name".to_string(),
                "updated_at".to_string()
            ])
        );
    }

    #[test]
    fn from_meta_invalid() {
        assert!(ReturningMode::from_string("").is_err());
    }
}
