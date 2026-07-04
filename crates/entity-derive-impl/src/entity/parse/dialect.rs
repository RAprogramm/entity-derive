// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
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
/// | `PostgreSQL` | ACID SQL | Row-level security, SSL, audit | Transactions |
/// | `ClickHouse` | OLAP | Multi-DC replication | Analytics |
/// | `MongoDB` | Document | E2E encryption, LDAP, RBAC | Documents |
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
    /// `PostgreSQL` - enterprise ACID database.
    ///
    /// - Placeholders: `$1, $2, $3, ...`
    /// - Client: `sqlx::PgPool`
    /// - Features: RETURNING, row-level security, JSONB
    #[default]
    Postgres,

    /// `ClickHouse` - high-performance OLAP database.
    ///
    /// - Placeholders: `$1, $2, $3, ...`
    /// - Client: `clickhouse::Client`
    /// - Features: columnar storage, real-time analytics
    ClickHouse,

    /// `MongoDB` - document database with enterprise security.
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
