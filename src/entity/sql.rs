// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! SQL implementation generation for the Entity derive macro.
//!
//! This module coordinates database-specific repository implementations.
//! Each dialect has its own submodule with specialized code generation.
//!
//! # Architecture
//!
//! ```text
//! sql.rs (coordinator)
//! ├── postgres.rs   - PostgreSQL via sqlx::PgPool
//! ├── clickhouse.rs - ClickHouse (planned)
//! └── mongodb.rs    - MongoDB (planned)
//! ```
//!
//! # Supported Dialects
//!
//! | Dialect | Feature | Client | Status |
//! |---------|---------|--------|--------|
//! | PostgreSQL | `postgres` | `sqlx::PgPool` | Stable |
//! | ClickHouse | `clickhouse` | `clickhouse::Client` | Planned |
//! | MongoDB | `mongodb` | `mongodb::Client` | Planned |

mod clickhouse;
mod mongodb;
mod postgres;

use proc_macro2::TokenStream;

use super::parse::{DatabaseDialect, EntityDef, SqlLevel};

/// Generate SQL implementation based on entity configuration.
///
/// Delegates to dialect-specific generators based on `#[entity(dialect =
/// "...")]`.
///
/// # Returns
///
/// - Empty `TokenStream` if `sql != "full"`
/// - Dialect-specific implementation otherwise
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql != SqlLevel::Full {
        return TokenStream::new();
    }

    match entity.dialect {
        DatabaseDialect::Postgres => postgres::generate(entity),
        DatabaseDialect::ClickHouse => clickhouse::generate(entity),
        DatabaseDialect::MongoDB => mongodb::generate(entity)
    }
}
