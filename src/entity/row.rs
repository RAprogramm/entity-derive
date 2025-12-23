// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! DB Row struct generation for the Entity derive macro.
//!
//! Generates Row struct with sqlx::FromRow derive.

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EntityDef, SqlLevel};

/// Generate Row struct for database queries.
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
        #[cfg_attr(feature = "db", derive(sqlx::FromRow))]
        #vis struct #row_name { #(#field_defs),* }
    }
}
