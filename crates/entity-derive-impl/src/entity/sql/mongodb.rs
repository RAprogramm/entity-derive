// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! MongoDB implementation for repository code generation.
//!
//! # Status
//!
//! Not yet implemented. Use `sql = "trait"` and implement manually.
//!
//! # Planned Features
//!
//! - Document-based operations (not SQL)
//! - BSON type mappings
//! - Aggregation pipeline support
//! - Index hint generation

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityDef;

/// Generate MongoDB repository implementation.
///
/// Currently generates a compile error directing users to implement manually.
pub fn generate(_entity: &EntityDef) -> TokenStream {
    quote! {
        compile_error!(
            "MongoDB support is not yet implemented. \
             Use `sql = \"trait\"` to generate only the trait, \
             then implement it manually for `mongodb::Client`."
        );
    }
}
