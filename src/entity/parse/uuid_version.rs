// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! UUID version configuration for ID generation.
//!
//! This module defines [`UuidVersion`], which controls which UUID version
//! is used for auto-generated primary keys.

use darling::FromMeta;

/// UUID version for ID generation.
///
/// Controls which UUID version is used for auto-generated primary keys.
///
/// # Variants
///
/// | Version | Method | Properties |
/// |---------|--------|------------|
/// | `V7` | `Uuid::now_v7()` | Time-ordered, sortable, default |
/// | `V4` | `Uuid::new_v4()` | Random, widely compatible |
///
/// # Examples
///
/// ```rust,ignore
/// // UUIDv7 (default) - time-ordered, best for databases
/// #[entity(table = "users")]
/// #[entity(table = "users", uuid = "v7")]
///
/// // UUIDv4 - random, for compatibility
/// #[entity(table = "users", uuid = "v4")]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UuidVersion {
    /// UUID version 7 - time-ordered.
    ///
    /// Uses `Uuid::now_v7()`. Recommended for database primary keys
    /// as it provides natural ordering by creation time.
    #[default]
    V7,

    /// UUID version 4 - random.
    ///
    /// Uses `Uuid::new_v4()`. Classic random UUID, widely supported.
    V4
}

impl FromMeta for UuidVersion {
    /// Parse UUID version from string attribute value.
    ///
    /// # Accepted Values
    ///
    /// - `"v7"`, `"7"` → [`UuidVersion::V7`]
    /// - `"v4"`, `"4"` → [`UuidVersion::V4`]
    ///
    /// Values are case-insensitive.
    ///
    /// # Errors
    ///
    /// Returns `darling::Error::unknown_value` for unrecognized values.
    fn from_string(value: &str) -> darling::Result<Self> {
        match value.to_lowercase().as_str() {
            "v7" | "7" => Ok(UuidVersion::V7),
            "v4" | "4" => Ok(UuidVersion::V4),
            _ => Err(darling::Error::unknown_value(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_v7() {
        assert_eq!(UuidVersion::default(), UuidVersion::V7);
    }

    #[test]
    fn from_meta_v7() {
        assert_eq!(UuidVersion::from_string("v7").unwrap(), UuidVersion::V7);
        assert_eq!(UuidVersion::from_string("7").unwrap(), UuidVersion::V7);
        assert_eq!(UuidVersion::from_string("V7").unwrap(), UuidVersion::V7);
    }

    #[test]
    fn from_meta_v4() {
        assert_eq!(UuidVersion::from_string("v4").unwrap(), UuidVersion::V4);
        assert_eq!(UuidVersion::from_string("4").unwrap(), UuidVersion::V4);
        assert_eq!(UuidVersion::from_string("V4").unwrap(), UuidVersion::V4);
    }

    #[test]
    fn from_meta_invalid() {
        assert!(UuidVersion::from_string("v1").is_err());
        assert!(UuidVersion::from_string("v5").is_err());
        assert!(UuidVersion::from_string("uuid7").is_err());
    }
}
