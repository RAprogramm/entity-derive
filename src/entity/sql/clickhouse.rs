// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! ClickHouse implementation for repository code generation.
//!
//! # Status
//!
//! Not yet implemented. Use `sql = "trait"` and implement manually.
//!
//! # Planned Features
//!
//! - Columnar storage optimizations
//! - Batch insert support
//! - MergeTree engine configuration
//! - Async insert mode

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityDef;

/// Generate ClickHouse repository implementation.
///
/// Currently generates a compile error directing users to implement manually.
pub fn generate(_entity: &EntityDef) -> TokenStream {
    quote! {
        compile_error!(
            "ClickHouse support is not yet implemented. \
             Use `sql = \"trait\"` to generate only the trait, \
             then implement it manually for `clickhouse::Client`."
        );
    }
}
