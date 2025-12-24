// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Database dialect configuration.
//!
//! This module defines [`DatabaseDialect`], which controls database-specific
//! SQL syntax generation (placeholders, pool types, etc.).

use darling::FromMeta;

/// Database dialect for code generation.
///
/// Controls database-specific syntax like parameter placeholders and client
/// types. The dialect is determined by compile-time feature flags.
///
/// # Supported Databases
///
/// | Dialect | Type | Security | Use Case |
/// |---------|------|----------|----------|
/// | PostgreSQL | ACID SQL | Row-level security, SSL, audit | Transactions |
/// | ClickHouse | OLAP | Multi-DC replication | Analytics |
/// | MongoDB | Document | E2E encryption, LDAP, RBAC | Documents |
///
/// # Examples
///
/// ```rust,ignore
/// #[entity(table = "users", dialect = "postgres")]
/// #[entity(table = "events", dialect = "clickhouse")]
/// #[entity(collection = "users", dialect = "mongodb")]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DatabaseDialect {
    /// PostgreSQL - enterprise ACID database.
    ///
    /// - Placeholders: `$1, $2, $3, ...`
    /// - Client: `sqlx::PgPool`
    /// - Features: RETURNING, row-level security, JSONB
    #[default]
    Postgres,

    /// ClickHouse - high-performance OLAP database.
    ///
    /// - Placeholders: `$1, $2, $3, ...`
    /// - Client: `clickhouse::Client`
    /// - Features: columnar storage, real-time analytics
    ClickHouse,

    /// MongoDB - document database with enterprise security.
    ///
    /// - Document-based (BSON)
    /// - Client: `mongodb::Client`
    /// - Features: E2E encryption, sharding, LDAP
    MongoDB
}

impl DatabaseDialect {
    /// Generate placeholder for parameter at given index (1-based).
    #[must_use]
    pub fn placeholder(&self, index: usize) -> String {
        match self {
            Self::Postgres | Self::ClickHouse => format!("${index}"),
            Self::MongoDB => format!("${index}") // For aggregation pipelines
        }
    }

    /// Generate comma-separated placeholders for given count.
    #[must_use]
    pub fn placeholders(&self, count: usize) -> String {
        (1..=count)
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Generate SET clause for UPDATE statement.
    #[must_use]
    pub fn set_clause(&self, fields: &[&str]) -> String {
        match self {
            Self::Postgres | Self::ClickHouse => fields
                .iter()
                .enumerate()
                .map(|(i, f)| format!("{} = ${}", f, i + 1))
                .collect::<Vec<_>>()
                .join(", "),
            Self::MongoDB => fields.join(", ") // MongoDB uses $set operator
        }
    }

    /// Get the client type path for this dialect.
    #[must_use]
    #[allow(dead_code)]
    pub fn client_type(&self) -> &'static str {
        match self {
            Self::Postgres => "sqlx::PgPool",
            Self::ClickHouse => "clickhouse::Client",
            Self::MongoDB => "mongodb::Client"
        }
    }

    /// Get the feature flag name for this dialect.
    #[must_use]
    pub fn feature_flag(&self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::ClickHouse => "clickhouse",
            Self::MongoDB => "mongodb"
        }
    }

    /// Check if RETURNING clause is supported.
    #[must_use]
    #[allow(dead_code)]
    pub fn supports_returning(&self) -> bool {
        matches!(self, Self::Postgres)
    }

    /// Check if this is a SQL-based database.
    #[must_use]
    #[allow(dead_code)]
    pub fn is_sql(&self) -> bool {
        matches!(self, Self::Postgres | Self::ClickHouse)
    }

    /// Check if this is a document database.
    #[must_use]
    #[allow(dead_code)]
    pub fn is_document(&self) -> bool {
        matches!(self, Self::MongoDB)
    }
}

impl FromMeta for DatabaseDialect {
    fn from_string(value: &str) -> darling::Result<Self> {
        match value.to_lowercase().as_str() {
            "postgres" | "postgresql" | "pg" => Ok(Self::Postgres),
            "clickhouse" | "ch" => Ok(Self::ClickHouse),
            "mongodb" | "mongo" => Ok(Self::MongoDB),
            _ => Err(darling::Error::unknown_value(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_placeholders() {
        let d = DatabaseDialect::Postgres;
        assert_eq!(d.placeholder(1), "$1");
        assert_eq!(d.placeholder(5), "$5");
        assert_eq!(d.placeholders(3), "$1, $2, $3");
        assert_eq!(d.placeholders(0), "");
    }

    #[test]
    fn clickhouse_placeholders() {
        let d = DatabaseDialect::ClickHouse;
        assert_eq!(d.placeholder(1), "$1");
        assert_eq!(d.placeholders(3), "$1, $2, $3");
    }

    #[test]
    fn mongodb_placeholders() {
        let d = DatabaseDialect::MongoDB;
        assert_eq!(d.placeholder(1), "$1");
        assert_eq!(d.placeholders(3), "$1, $2, $3");
    }

    #[test]
    fn set_clause_postgres() {
        let d = DatabaseDialect::Postgres;
        let fields = ["name", "email"];
        assert_eq!(d.set_clause(&fields), "name = $1, email = $2");
    }

    #[test]
    fn set_clause_clickhouse() {
        let d = DatabaseDialect::ClickHouse;
        let fields = ["name", "email"];
        assert_eq!(d.set_clause(&fields), "name = $1, email = $2");
    }

    #[test]
    fn set_clause_mongodb() {
        let d = DatabaseDialect::MongoDB;
        let fields = ["name", "email"];
        assert_eq!(d.set_clause(&fields), "name, email");
    }

    #[test]
    fn client_types() {
        assert_eq!(DatabaseDialect::Postgres.client_type(), "sqlx::PgPool");
        assert_eq!(
            DatabaseDialect::ClickHouse.client_type(),
            "clickhouse::Client"
        );
        assert_eq!(DatabaseDialect::MongoDB.client_type(), "mongodb::Client");
    }

    #[test]
    fn feature_flags() {
        assert_eq!(DatabaseDialect::Postgres.feature_flag(), "postgres");
        assert_eq!(DatabaseDialect::ClickHouse.feature_flag(), "clickhouse");
        assert_eq!(DatabaseDialect::MongoDB.feature_flag(), "mongodb");
    }

    #[test]
    fn is_sql() {
        assert!(DatabaseDialect::Postgres.is_sql());
        assert!(DatabaseDialect::ClickHouse.is_sql());
        assert!(!DatabaseDialect::MongoDB.is_sql());
    }

    #[test]
    fn is_document() {
        assert!(!DatabaseDialect::Postgres.is_document());
        assert!(!DatabaseDialect::ClickHouse.is_document());
        assert!(DatabaseDialect::MongoDB.is_document());
    }

    #[test]
    fn supports_returning() {
        assert!(DatabaseDialect::Postgres.supports_returning());
        assert!(!DatabaseDialect::ClickHouse.supports_returning());
        assert!(!DatabaseDialect::MongoDB.supports_returning());
    }

    #[test]
    fn from_meta_postgres() {
        assert_eq!(
            DatabaseDialect::from_string("postgres").unwrap(),
            DatabaseDialect::Postgres
        );
        assert_eq!(
            DatabaseDialect::from_string("POSTGRESQL").unwrap(),
            DatabaseDialect::Postgres
        );
        assert_eq!(
            DatabaseDialect::from_string("pg").unwrap(),
            DatabaseDialect::Postgres
        );
    }

    #[test]
    fn from_meta_clickhouse() {
        assert_eq!(
            DatabaseDialect::from_string("clickhouse").unwrap(),
            DatabaseDialect::ClickHouse
        );
        assert_eq!(
            DatabaseDialect::from_string("CH").unwrap(),
            DatabaseDialect::ClickHouse
        );
    }

    #[test]
    fn from_meta_mongodb() {
        assert_eq!(
            DatabaseDialect::from_string("mongodb").unwrap(),
            DatabaseDialect::MongoDB
        );
        assert_eq!(
            DatabaseDialect::from_string("MONGO").unwrap(),
            DatabaseDialect::MongoDB
        );
    }

    #[test]
    fn from_meta_invalid() {
        assert!(DatabaseDialect::from_string("mysql").is_err());
        assert!(DatabaseDialect::from_string("sqlite").is_err());
        assert!(DatabaseDialect::from_string("oracle").is_err());
    }

    #[test]
    fn default_is_postgres() {
        assert_eq!(DatabaseDialect::default(), DatabaseDialect::Postgres);
    }
}
