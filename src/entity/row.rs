// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Database row struct generation.
//!
//! Generates a `{Name}Row` struct that maps directly to database query results.
//! This struct implements `sqlx::FromRow` for automatic deserialization.
//!
//! # Generated Struct
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! #[derive(Debug, Clone)]
//! #[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
//! pub struct UserRow {
//!     pub id: Uuid,
//!     pub name: String,
//!     pub email: String,
//!     pub created_at: DateTime<Utc>,
//! }
//! ```
//!
//! # Purpose
//!
//! The Row struct serves as an intermediate type between raw database results
//! and the domain entity. This separation allows:
//!
//! - **Type safety**: Database columns map to explicit Rust types
//! - **Decoupling**: Entity can evolve independently of DB schema
//! - **Validation**: Conversion from Row to Entity can include validation
//!
//! # Field Inclusion
//!
//! Unlike DTOs, the Row struct includes ALL fields from the entity:
//!
//! | Field Type | In Row | Reason |
//! |------------|--------|--------|
//! | `#[id]` | Yes | Primary key from DB |
//! | `#[auto]` | Yes | Timestamps from DB |
//! | `#[field(skip)]` | Yes | Still stored in DB |
//! | Regular fields | Yes | All data columns |
//!
//! # Conditional Compilation
//!
//! The `sqlx::FromRow` derive is gated behind `#[cfg(feature = "postgres")]`.
//! This allows using the crate without sqlx for DTO-only scenarios.

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EntityDef, SqlLevel};

/// Generates the `{Name}Row` struct for database query results.
///
/// Returns an empty `TokenStream` if `sql = "none"` is specified,
/// as Row structs are only needed for database operations.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let row_name = entity.ident_with("", "Row");
    let field_defs = entity.all_fields().iter().map(|f| {
        let name = f.name();
        let ty = f.ty();
        quote! { pub #name: #ty }
    });

    quote! {
        #[derive(Debug, Clone)]
        #[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
        #vis struct #row_name { #(#field_defs),* }
    }
}
