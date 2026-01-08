// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Composite index definitions for entity-level indexes.
//!
//! Parsed from `#[entity(index(...))]` and `#[entity(unique_index(...))]`
//! attributes.
//!
//! # Examples
//!
//! ```rust,ignore
//! #[entity(
//!     table = "users",
//!     index(name, email),                    // Default btree composite
//!     index(type = "gin", tags),             // GIN index
//!     unique_index(tenant_id, email),        // Unique composite
//!     index(name = "idx_custom", status),    // Named index
//!     index(status, where = "active = true") // Partial index
//! )]
//! pub struct User { ... }
//! ```

use crate::entity::parse::field::IndexType;

/// Composite index definition from entity-level attributes.
///
/// Represents an index spanning one or more columns.
#[derive(Debug, Clone)]
pub struct CompositeIndexDef {
    /// Index name. Auto-generated if not specified.
    ///
    /// Format: `idx_{table}_{col1}_{col2}_...`
    pub name: Option<String>,

    /// Column names included in the index.
    pub columns: Vec<String>,

    /// Index type (btree, hash, gin, gist, brin).
    pub index_type: IndexType,

    /// Whether this is a unique index.
    pub unique: bool,

    /// WHERE clause for partial index (raw SQL).
    ///
    /// Example: `"active = true"`, `"deleted_at IS NULL"`
    pub where_clause: Option<String>
}

impl CompositeIndexDef {
    /// Create a new non-unique btree index.
    #[cfg(test)]
    #[must_use]
    pub fn new(columns: Vec<String>) -> Self {
        Self {
            name: None,
            columns,
            index_type: IndexType::default(),
            unique: false,
            where_clause: None
        }
    }

    /// Create a new unique btree index.
    #[cfg(test)]
    #[must_use]
    pub fn unique(columns: Vec<String>) -> Self {
        Self {
            name: None,
            columns,
            index_type: IndexType::default(),
            unique: true,
            where_clause: None
        }
    }

    /// Set the index name.
    #[cfg(test)]
    #[must_use]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the index type.
    #[cfg(test)]
    #[must_use]
    pub fn with_type(mut self, index_type: IndexType) -> Self {
        self.index_type = index_type;
        self
    }

    /// Set the WHERE clause for partial index.
    #[cfg(test)]
    #[must_use]
    pub fn with_where(mut self, where_clause: String) -> Self {
        self.where_clause = Some(where_clause);
        self
    }

    /// Generate the default index name.
    ///
    /// Format: `idx_{table}_{col1}_{col2}_...`
    #[must_use]
    pub fn default_name(&self, table: &str) -> String {
        format!("idx_{}_{}", table, self.columns.join("_"))
    }

    /// Get the index name, using default if not set.
    #[must_use]
    pub fn name_or_default(&self, table: &str) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| self.default_name(table))
    }

    /// Check if this is a partial index.
    #[cfg(test)]
    #[must_use]
    pub fn is_partial(&self) -> bool {
        self.where_clause.is_some()
    }
}

/// Parse index attributes from entity-level meta list.
///
/// Handles both `index(...)` and `unique_index(...)` forms.
///
/// # Syntax
///
/// ```text
/// index(col1, col2, ...)
/// index(type = "gin", col1, col2)
/// index(name = "idx_name", col1, col2)
/// index(col1, where = "condition")
/// unique_index(col1, col2)
/// ```
#[allow(dead_code)] // Will be used when full index parsing is integrated
pub fn parse_index_meta(
    meta: syn::meta::ParseNestedMeta<'_>,
    unique: bool
) -> syn::Result<CompositeIndexDef> {
    let mut columns = Vec::new();
    let mut name = None;
    let mut index_type = IndexType::default();
    let mut where_clause = None;

    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("type") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            index_type = IndexType::from_str(&value.value()).unwrap_or_default();
        } else if nested.path.is_ident("name") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            name = Some(value.value());
        } else if nested.path.is_ident("where") {
            let _: syn::Token![=] = nested.input.parse()?;
            let value: syn::LitStr = nested.input.parse()?;
            where_clause = Some(value.value());
        } else {
            // Assume it's a column name
            let col = nested
                .path
                .get_ident()
                .map(|i| i.to_string())
                .ok_or_else(|| nested.error("expected column name"))?;
            columns.push(col);
        }
        Ok(())
    })?;

    if columns.is_empty() {
        return Err(meta.error("index must have at least one column"));
    }

    Ok(CompositeIndexDef {
        name,
        columns,
        index_type,
        unique,
        where_clause
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_btree_index() {
        let idx = CompositeIndexDef::new(vec!["name".to_string(), "email".to_string()]);
        assert_eq!(idx.columns, vec!["name", "email"]);
        assert_eq!(idx.index_type, IndexType::BTree);
        assert!(!idx.unique);
        assert!(idx.name.is_none());
        assert!(idx.where_clause.is_none());
    }

    #[test]
    fn unique_creates_unique_index() {
        let idx = CompositeIndexDef::unique(vec!["tenant_id".to_string(), "email".to_string()]);
        assert!(idx.unique);
        assert_eq!(idx.index_type, IndexType::BTree);
    }

    #[test]
    fn with_name_sets_name() {
        let idx =
            CompositeIndexDef::new(vec!["status".to_string()]).with_name("idx_custom".to_string());
        assert_eq!(idx.name, Some("idx_custom".to_string()));
    }

    #[test]
    fn with_type_sets_type() {
        let idx = CompositeIndexDef::new(vec!["tags".to_string()]).with_type(IndexType::Gin);
        assert_eq!(idx.index_type, IndexType::Gin);
    }

    #[test]
    fn with_where_sets_partial() {
        let idx = CompositeIndexDef::new(vec!["status".to_string()])
            .with_where("active = true".to_string());
        assert!(idx.is_partial());
        assert_eq!(idx.where_clause, Some("active = true".to_string()));
    }

    #[test]
    fn default_name_format() {
        let idx = CompositeIndexDef::new(vec!["name".to_string(), "email".to_string()]);
        assert_eq!(idx.default_name("users"), "idx_users_name_email");
    }

    #[test]
    fn name_or_default_uses_custom() {
        let idx =
            CompositeIndexDef::new(vec!["status".to_string()]).with_name("my_idx".to_string());
        assert_eq!(idx.name_or_default("users"), "my_idx");
    }

    #[test]
    fn name_or_default_uses_generated() {
        let idx = CompositeIndexDef::new(vec!["status".to_string()]);
        assert_eq!(idx.name_or_default("users"), "idx_users_status");
    }
}
