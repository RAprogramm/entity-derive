// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Column-level database configuration for migrations.
//!
//! Controls database-specific constraints, indexes, and type mappings.
//!
//! # Supported Attributes
//!
//! | Attribute | Example | SQL |
//! |-----------|---------|-----|
//! | `unique` | `#[column(unique)]` | `UNIQUE` |
//! | `index` | `#[column(index)]` | `CREATE INDEX` (btree) |
//! | `index` | `#[column(index = "gin")]` | `CREATE INDEX USING gin` |
//! | `default` | `#[column(default = "true")]` | `DEFAULT true` |
//! | `check` | `#[column(check = "age >= 0")]` | `CHECK (age >= 0)` |
//! | `varchar` | `#[column(varchar = 255)]` | `VARCHAR(255)` |
//! | `sql_type` | `#[column(sql_type = "JSONB")]` | Explicit type |
//! | `nullable` | `#[column(nullable)]` | Allow NULL |
//! | `name` | `#[column(name = "user_name")]` | Custom column name |

use syn::{Attribute, Meta};

/// Index type for database indexes.
///
/// PostgreSQL supports multiple index types optimized for different use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndexType {
    /// B-tree index (default). Best for equality and range queries.
    #[default]
    BTree,

    /// Hash index. Only for equality comparisons.
    Hash,

    /// GIN (Generalized Inverted Index). For array/JSONB containment.
    Gin,

    /// GiST (Generalized Search Tree). For geometric/full-text search.
    Gist,

    /// BRIN (Block Range Index). For large sequential data.
    Brin
}

impl IndexType {
    /// Parse index type from string.
    ///
    /// Returns `None` for unrecognized values.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "btree" | "b-tree" => Some(Self::BTree),
            "hash" => Some(Self::Hash),
            "gin" => Some(Self::Gin),
            "gist" => Some(Self::Gist),
            "brin" => Some(Self::Brin),
            _ => None
        }
    }

    /// Get SQL USING clause for this index type.
    ///
    /// Returns empty string for btree (default).
    #[must_use]
    pub fn as_sql_using(&self) -> &'static str {
        match self {
            Self::BTree => "",
            Self::Hash => " USING hash",
            Self::Gin => " USING gin",
            Self::Gist => " USING gist",
            Self::Brin => " USING brin"
        }
    }
}

/// Referential action for foreign key ON DELETE/ON UPDATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferentialAction {
    /// Delete/update child rows when parent is deleted/updated.
    Cascade,

    /// Set foreign key to NULL.
    SetNull,

    /// Set foreign key to default value.
    SetDefault,

    /// Prevent deletion/update if children exist (deferred check).
    Restrict,

    /// Prevent deletion/update if children exist (immediate check).
    NoAction
}

impl ReferentialAction {
    /// Parse referential action from string.
    ///
    /// Returns `None` for unrecognized values.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace([' ', '_'], "").as_str() {
            "cascade" => Some(Self::Cascade),
            "setnull" => Some(Self::SetNull),
            "setdefault" => Some(Self::SetDefault),
            "restrict" => Some(Self::Restrict),
            "noaction" => Some(Self::NoAction),
            _ => None
        }
    }

    /// Get SQL representation of this action.
    #[must_use]
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Cascade => "CASCADE",
            Self::SetNull => "SET NULL",
            Self::SetDefault => "SET DEFAULT",
            Self::Restrict => "RESTRICT",
            Self::NoAction => "NO ACTION"
        }
    }
}

/// Column-level database configuration.
///
/// Parsed from `#[column(...)]` attributes on entity fields.
///
/// # Example
///
/// ```rust,ignore
/// #[column(unique, index = "btree", default = "true")]
/// pub is_active: bool,
///
/// #[column(varchar = 100, check = "length(name) > 0")]
/// pub name: String,
///
/// #[column(sql_type = "JSONB")]
/// pub metadata: serde_json::Value,
/// ```
#[derive(Debug, Clone, Default)]
pub struct ColumnConfig {
    /// UNIQUE constraint on this column.
    pub unique: bool,

    /// Index type if indexed. `None` means no index.
    pub index: Option<IndexType>,

    /// DEFAULT value expression (raw SQL).
    ///
    /// Examples: `"true"`, `"NOW()"`, `"'pending'"`.
    pub default: Option<String>,

    /// CHECK constraint expression (raw SQL).
    ///
    /// Example: `"age >= 0"`, `"length(name) > 0"`.
    pub check: Option<String>,

    /// VARCHAR length. Converts `String` to `VARCHAR(n)`.
    pub varchar: Option<usize>,

    /// Explicit SQL type override.
    ///
    /// Bypasses automatic type mapping.
    pub sql_type: Option<String>,

    /// Explicitly allow NULL even for non-Option types.
    pub nullable: bool,

    /// Custom column name. Defaults to field name.
    pub name: Option<String>
}

impl ColumnConfig {
    /// Parse column config from `#[column(...)]` attribute.
    ///
    /// # Recognized Options
    ///
    /// - `unique` — Add UNIQUE constraint
    /// - `index` — Create btree index
    /// - `index = "type"` — Create index of specified type
    /// - `default = "expr"` — Set DEFAULT value
    /// - `check = "expr"` — Add CHECK constraint
    /// - `varchar = N` — Use VARCHAR(N) instead of TEXT
    /// - `sql_type = "TYPE"` — Override SQL type
    /// - `nullable` — Allow NULL
    /// - `name = "col"` — Custom column name
    pub fn from_attr(attr: &Attribute) -> Self {
        let mut config = Self::default();

        if let Meta::List(meta_list) = &attr.meta {
            let _ = meta_list.parse_nested_meta(|meta| {
                if meta.path.is_ident("unique") {
                    config.unique = true;
                } else if meta.path.is_ident("index") {
                    if meta.input.peek(syn::Token![=]) {
                        let _: syn::Token![=] = meta.input.parse()?;
                        let value: syn::LitStr = meta.input.parse()?;
                        config.index =
                            Some(IndexType::from_str(&value.value()).unwrap_or_default());
                    } else {
                        config.index = Some(IndexType::default());
                    }
                } else if meta.path.is_ident("default") {
                    let _: syn::Token![=] = meta.input.parse()?;
                    let value: syn::LitStr = meta.input.parse()?;
                    config.default = Some(value.value());
                } else if meta.path.is_ident("check") {
                    let _: syn::Token![=] = meta.input.parse()?;
                    let value: syn::LitStr = meta.input.parse()?;
                    config.check = Some(value.value());
                } else if meta.path.is_ident("varchar") {
                    let _: syn::Token![=] = meta.input.parse()?;
                    let value: syn::LitInt = meta.input.parse()?;
                    config.varchar = value.base10_parse().ok();
                } else if meta.path.is_ident("sql_type") {
                    let _: syn::Token![=] = meta.input.parse()?;
                    let value: syn::LitStr = meta.input.parse()?;
                    config.sql_type = Some(value.value());
                } else if meta.path.is_ident("nullable") {
                    config.nullable = true;
                } else if meta.path.is_ident("name") {
                    let _: syn::Token![=] = meta.input.parse()?;
                    let value: syn::LitStr = meta.input.parse()?;
                    config.name = Some(value.value());
                }
                Ok(())
            });
        }

        config
    }

    /// Check if this column has any constraints.
    #[must_use]
    #[allow(dead_code)] // Public API for future use
    pub fn has_constraints(&self) -> bool {
        self.unique || self.check.is_some()
    }

    /// Check if this column should be indexed.
    #[must_use]
    pub fn has_index(&self) -> bool {
        self.index.is_some()
    }

    /// Get the column name, using custom name if set.
    #[must_use]
    pub fn column_name<'a>(&'a self, field_name: &'a str) -> &'a str {
        self.name.as_deref().unwrap_or(field_name)
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    fn parse_column_attr(tokens: proc_macro2::TokenStream) -> ColumnConfig {
        let attr: Attribute = parse_quote!(#[column(#tokens)]);
        ColumnConfig::from_attr(&attr)
    }

    #[test]
    fn default_is_empty() {
        let config = ColumnConfig::default();
        assert!(!config.unique);
        assert!(config.index.is_none());
        assert!(config.default.is_none());
        assert!(config.check.is_none());
        assert!(config.varchar.is_none());
        assert!(config.sql_type.is_none());
        assert!(!config.nullable);
        assert!(config.name.is_none());
    }

    #[test]
    fn parse_unique() {
        let config = parse_column_attr(quote! { unique });
        assert!(config.unique);
    }

    #[test]
    fn parse_index_default() {
        let config = parse_column_attr(quote! { index });
        assert_eq!(config.index, Some(IndexType::BTree));
    }

    #[test]
    fn parse_index_gin() {
        let config = parse_column_attr(quote! { index = "gin" });
        assert_eq!(config.index, Some(IndexType::Gin));
    }

    #[test]
    fn parse_index_hash() {
        let config = parse_column_attr(quote! { index = "hash" });
        assert_eq!(config.index, Some(IndexType::Hash));
    }

    #[test]
    fn parse_default_value() {
        let config = parse_column_attr(quote! { default = "true" });
        assert_eq!(config.default, Some("true".to_string()));
    }

    #[test]
    fn parse_default_now() {
        let config = parse_column_attr(quote! { default = "NOW()" });
        assert_eq!(config.default, Some("NOW()".to_string()));
    }

    #[test]
    fn parse_check_constraint() {
        let config = parse_column_attr(quote! { check = "age >= 0" });
        assert_eq!(config.check, Some("age >= 0".to_string()));
    }

    #[test]
    fn parse_varchar() {
        let config = parse_column_attr(quote! { varchar = 255 });
        assert_eq!(config.varchar, Some(255));
    }

    #[test]
    fn parse_sql_type() {
        let config = parse_column_attr(quote! { sql_type = "JSONB" });
        assert_eq!(config.sql_type, Some("JSONB".to_string()));
    }

    #[test]
    fn parse_nullable() {
        let config = parse_column_attr(quote! { nullable });
        assert!(config.nullable);
    }

    #[test]
    fn parse_custom_name() {
        let config = parse_column_attr(quote! { name = "user_name" });
        assert_eq!(config.name, Some("user_name".to_string()));
    }

    #[test]
    fn parse_multiple_attrs() {
        let config = parse_column_attr(quote! { unique, index = "btree", default = "true" });
        assert!(config.unique);
        assert_eq!(config.index, Some(IndexType::BTree));
        assert_eq!(config.default, Some("true".to_string()));
    }

    #[test]
    fn has_constraints_check() {
        let config = parse_column_attr(quote! { unique });
        assert!(config.has_constraints());

        let config2 = parse_column_attr(quote! { check = "x > 0" });
        assert!(config2.has_constraints());

        let config3 = ColumnConfig::default();
        assert!(!config3.has_constraints());
    }

    #[test]
    fn has_index_check() {
        let config = parse_column_attr(quote! { index });
        assert!(config.has_index());

        let config2 = ColumnConfig::default();
        assert!(!config2.has_index());
    }

    #[test]
    fn column_name_default() {
        let config = ColumnConfig::default();
        assert_eq!(config.column_name("email"), "email");
    }

    #[test]
    fn column_name_custom() {
        let config = parse_column_attr(quote! { name = "user_email" });
        assert_eq!(config.column_name("email"), "user_email");
    }

    #[test]
    fn index_type_as_sql() {
        assert_eq!(IndexType::BTree.as_sql_using(), "");
        assert_eq!(IndexType::Hash.as_sql_using(), " USING hash");
        assert_eq!(IndexType::Gin.as_sql_using(), " USING gin");
        assert_eq!(IndexType::Gist.as_sql_using(), " USING gist");
        assert_eq!(IndexType::Brin.as_sql_using(), " USING brin");
    }

    #[test]
    fn index_type_from_str_all() {
        assert_eq!(IndexType::from_str("btree"), Some(IndexType::BTree));
        assert_eq!(IndexType::from_str("b-tree"), Some(IndexType::BTree));
        assert_eq!(IndexType::from_str("BTREE"), Some(IndexType::BTree));
        assert_eq!(IndexType::from_str("hash"), Some(IndexType::Hash));
        assert_eq!(IndexType::from_str("HASH"), Some(IndexType::Hash));
        assert_eq!(IndexType::from_str("gin"), Some(IndexType::Gin));
        assert_eq!(IndexType::from_str("GIN"), Some(IndexType::Gin));
        assert_eq!(IndexType::from_str("gist"), Some(IndexType::Gist));
        assert_eq!(IndexType::from_str("GIST"), Some(IndexType::Gist));
        assert_eq!(IndexType::from_str("brin"), Some(IndexType::Brin));
        assert_eq!(IndexType::from_str("BRIN"), Some(IndexType::Brin));
        assert_eq!(IndexType::from_str("invalid"), None);
        assert_eq!(IndexType::from_str("unknown"), None);
    }

    #[test]
    fn parse_index_gist() {
        let config = parse_column_attr(quote! { index = "gist" });
        assert_eq!(config.index, Some(IndexType::Gist));
    }

    #[test]
    fn parse_index_brin() {
        let config = parse_column_attr(quote! { index = "brin" });
        assert_eq!(config.index, Some(IndexType::Brin));
    }

    #[test]
    fn parse_index_unknown_defaults_to_btree() {
        let config = parse_column_attr(quote! { index = "unknown" });
        assert_eq!(config.index, Some(IndexType::BTree));
    }

    #[test]
    fn referential_action_from_str() {
        assert_eq!(
            ReferentialAction::from_str("cascade"),
            Some(ReferentialAction::Cascade)
        );
        assert_eq!(
            ReferentialAction::from_str("SET NULL"),
            Some(ReferentialAction::SetNull)
        );
        assert_eq!(
            ReferentialAction::from_str("set_default"),
            Some(ReferentialAction::SetDefault)
        );
        assert_eq!(
            ReferentialAction::from_str("RESTRICT"),
            Some(ReferentialAction::Restrict)
        );
        assert_eq!(
            ReferentialAction::from_str("no action"),
            Some(ReferentialAction::NoAction)
        );
        assert_eq!(ReferentialAction::from_str("invalid"), None);
    }

    #[test]
    fn referential_action_as_sql() {
        assert_eq!(ReferentialAction::Cascade.as_sql(), "CASCADE");
        assert_eq!(ReferentialAction::SetNull.as_sql(), "SET NULL");
        assert_eq!(ReferentialAction::SetDefault.as_sql(), "SET DEFAULT");
        assert_eq!(ReferentialAction::Restrict.as_sql(), "RESTRICT");
        assert_eq!(ReferentialAction::NoAction.as_sql(), "NO ACTION");
    }
}
