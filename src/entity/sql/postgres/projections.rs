// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Projection method generators for PostgreSQL.
//!
//! Generates optimized SELECT methods for entity projections defined with
//! `#[projection(Name: field1, field2, ...)]`.
//!
//! # Example
//!
//! For an entity with `#[projection(Public: id, name)]`:
//!
//! ```rust,ignore
//! async fn find_by_id_public(&self, id: Uuid) -> Result<Option<UserPublic>, Self::Error>;
//! ```
//!
//! # SQL Optimization
//!
//! Projections only SELECT the specified columns, reducing network transfer
//! and database load for read-heavy applications.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;
use crate::entity::parse::ProjectionDef;

impl Context<'_> {
    /// Generate all projection methods.
    ///
    /// Creates a `find_by_id_{projection_name}` method for each projection.
    pub fn projection_methods(&self) -> TokenStream {
        let methods: Vec<TokenStream> = self
            .entity
            .projections
            .iter()
            .map(|proj| self.projection_method(proj))
            .collect();

        quote! { #(#methods)* }
    }

    /// Generate a single projection method.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT field1, field2, ... FROM schema.table WHERE id = $1
    /// ```
    ///
    /// Only selects the columns specified in the projection definition.
    fn projection_method(&self, proj: &ProjectionDef) -> TokenStream {
        let entity_name = self.entity_name;
        let proj_snake = proj.name.to_string().to_case(Case::Snake);
        let method_name = format_ident!("find_by_id_{}", proj_snake);
        let proj_type = format_ident!("{}{}", entity_name, proj.name);
        let id_name = self.id_name;
        let id_type = self.id_type;
        let table = &self.table;
        let placeholder = self.dialect.placeholder(1);

        let columns_str: String = proj
            .fields
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        quote! {
            async fn #method_name(&self, id: #id_type) -> Result<Option<#proj_type>, Self::Error> {
                let row = sqlx::query_as::<_, #proj_type>(
                    &format!("SELECT {} FROM {} WHERE {} = {}", #columns_str, #table, stringify!(#id_name), #placeholder)
                ).bind(&id).fetch_optional(self).await?;
                Ok(row)
            }
        }
    }
}
