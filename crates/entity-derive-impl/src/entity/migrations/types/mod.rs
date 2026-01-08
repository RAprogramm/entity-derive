// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Type mapping from Rust to database-specific SQL types.
//!
//! This module provides traits and implementations for mapping Rust types
//! to their SQL equivalents during migration generation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                       Type Mapping System                           │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Rust Type              TypeMapper            SQL Type              │
//! │                                                                     │
//! │  Uuid            ──►  PostgresMapper  ──►   UUID                   │
//! │  String          ──►                  ──►   TEXT / VARCHAR(n)      │
//! │  i32             ──►                  ──►   INTEGER                │
//! │  DateTime<Utc>   ──►                  ──►   TIMESTAMPTZ            │
//! │  Option<T>       ──►                  ──►   T (nullable)           │
//! │  Vec<T>          ──►                  ──►   T[]                    │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

mod postgres;

pub use postgres::PostgresTypeMapper;

use syn::Type;

use crate::entity::parse::field::ColumnConfig;

/// Mapped SQL type representation.
#[derive(Debug, Clone)]
pub struct SqlType {
    /// SQL type name (e.g., "UUID", "TEXT", "INTEGER").
    pub name: String,

    /// Whether this type allows NULL values.
    pub nullable: bool,

    /// Array dimension (0 = scalar, 1 = T[], 2 = T[][], etc.).
    pub array_dim: usize
}

impl SqlType {
    /// Create a non-nullable SQL type.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name:      name.into(),
            nullable:  false,
            array_dim: 0
        }
    }

    /// Create a nullable SQL type.
    #[must_use]
    pub fn nullable(name: impl Into<String>) -> Self {
        Self {
            name:      name.into(),
            nullable:  true,
            array_dim: 0
        }
    }

    /// Get the full SQL type string with array suffix.
    #[must_use]
    pub fn to_sql_string(&self) -> String {
        if self.array_dim > 0 {
            format!("{}{}", self.name, "[]".repeat(self.array_dim))
        } else {
            self.name.clone()
        }
    }
}

/// Trait for mapping Rust types to SQL types.
///
/// Implement this trait for each database dialect.
pub trait TypeMapper {
    /// Map a Rust type to its SQL representation.
    ///
    /// # Arguments
    ///
    /// * `ty` - The Rust type from syn
    /// * `column` - Column configuration with overrides
    ///
    /// # Returns
    ///
    /// `SqlType` with name, nullable flag, and array dimension.
    fn map_type(&self, ty: &Type, column: &ColumnConfig) -> SqlType;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_type_new() {
        let ty = SqlType::new("INTEGER");
        assert_eq!(ty.name, "INTEGER");
        assert!(!ty.nullable);
        assert_eq!(ty.array_dim, 0);
    }

    #[test]
    fn sql_type_nullable() {
        let ty = SqlType::nullable("TEXT");
        assert!(ty.nullable);
    }

    #[test]
    fn sql_type_to_sql_string_scalar() {
        let ty = SqlType::new("UUID");
        assert_eq!(ty.to_sql_string(), "UUID");
    }

    #[test]
    fn sql_type_to_sql_string_array() {
        let ty = SqlType {
            name:      "TEXT".to_string(),
            nullable:  false,
            array_dim: 1
        };
        assert_eq!(ty.to_sql_string(), "TEXT[]");
    }

    #[test]
    fn sql_type_to_sql_string_2d_array() {
        let ty = SqlType {
            name:      "INTEGER".to_string(),
            nullable:  false,
            array_dim: 2
        };
        assert_eq!(ty.to_sql_string(), "INTEGER[][]");
    }
}
