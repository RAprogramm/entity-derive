// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Migration generation for entity-derive.
//!
//! Generates `MIGRATION_UP` and `MIGRATION_DOWN` constants containing
//! SQL DDL statements for creating/dropping tables.
//!
//! # Features
//!
//! - Full type mapping (Rust → PostgreSQL)
//! - Column constraints (UNIQUE, CHECK, DEFAULT)
//! - Indexes (btree, hash, gin, gist, brin)
//! - Foreign keys with ON DELETE actions
//! - Composite indexes
//!
//! # Usage
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users", migrations)]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!
//!     #[column(unique, index)]
//!     pub email: String,
//! }
//!
//! // Apply migration:
//! sqlx::query(User::MIGRATION_UP).execute(&pool).await?;
//! ```

mod postgres;
pub mod types;

use proc_macro2::TokenStream;

use super::parse::{DatabaseDialect, EntityDef};

/// Generate migration constants based on entity configuration.
///
/// Returns empty `TokenStream` if migrations are not enabled.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.migrations {
        return TokenStream::new();
    }

    match entity.dialect {
        DatabaseDialect::Postgres => postgres::generate(entity),
        DatabaseDialect::ClickHouse => TokenStream::new(), // TODO: future
        DatabaseDialect::MongoDB => TokenStream::new()     // N/A for document DB
    }
}
