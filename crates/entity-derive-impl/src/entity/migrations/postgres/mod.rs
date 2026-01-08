// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! PostgreSQL migration generation.
//!
//! Generates `MIGRATION_UP` and `MIGRATION_DOWN` constants for PostgreSQL.

mod ddl;

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityDef;
use crate::utils::marker;

/// Generate migration constants for PostgreSQL.
///
/// # Generated Code
///
/// ```rust,ignore
/// impl User {
///     pub const MIGRATION_UP: &'static str = "CREATE TABLE...";
///     pub const MIGRATION_DOWN: &'static str = "DROP TABLE...";
/// }
/// ```
pub fn generate(entity: &EntityDef) -> TokenStream {
    let entity_name = entity.name();
    let vis = &entity.vis;

    let up_sql = ddl::generate_up(entity);
    let down_sql = ddl::generate_down(entity);

    let marker = marker::generated();

    quote! {
        #marker
        impl #entity_name {
            /// SQL migration to create this entity's table, indexes, and constraints.
            ///
            /// # Usage
            ///
            /// ```rust,ignore
            /// sqlx::query(User::MIGRATION_UP).execute(&pool).await?;
            /// ```
            #vis const MIGRATION_UP: &'static str = #up_sql;

            /// SQL migration to drop this entity's table.
            ///
            /// Uses CASCADE to drop dependent objects.
            #vis const MIGRATION_DOWN: &'static str = #down_sql;
        }
    }
}
