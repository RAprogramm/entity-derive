// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Query method generator for PostgreSQL.
//!
//! Generates the `query` method that provides type-safe filtering using
//! the entity's Query struct (generated from `#[filter]` attributes).
//!
//! # Generated SQL
//!
//! ```sql
//! SELECT col1, col2, ... FROM schema.table
//! WHERE condition1 AND condition2 AND ...
//! ORDER BY id DESC
//! LIMIT $n OFFSET $m
//! ```
//!
//! # Dynamic WHERE Clause
//!
//! The WHERE clause is built at runtime based on which filter fields
//! are set in the query struct. Only `Some` values generate conditions.

use proc_macro2::TokenStream;
use quote::quote;

use super::{
    context::Context,
    helpers::{generate_query_bindings, generate_where_conditions}
};

impl Context<'_> {
    /// Generate the `query` method implementation.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` if entity has no filter fields.
    ///
    /// # Generated Code
    ///
    /// ```rust,ignore
    /// async fn query(&self, query: UserQuery) -> Result<Vec<User>, Self::Error> {
    ///     let mut conditions: Vec<String> = Vec::new();
    ///     let mut param_idx: usize = 1;
    ///
    ///     // Build conditions based on filter fields
    ///     if query.name.is_some() {
    ///         conditions.push(format!("name = ${}", param_idx));
    ///         param_idx += 1;
    ///     }
    ///     // ... more conditions
    ///
    ///     let where_clause = if conditions.is_empty() {
    ///         String::new()
    ///     } else {
    ///         format!("WHERE {}", conditions.join(" AND "))
    ///     };
    ///
    ///     let sql = format!("SELECT ... FROM ... {} ORDER BY ...", where_clause);
    ///
    ///     let mut q = sqlx::query_as::<_, UserRow>(&sql);
    ///     // Bind filter values
    ///     if let Some(ref v) = query.name {
    ///         q = q.bind(v);
    ///     }
    ///     // ... more bindings
    ///
    ///     q = q.bind(query.limit.unwrap_or(100)).bind(query.offset.unwrap_or(0));
    ///     let rows = q.fetch_all(self).await?;
    ///     Ok(rows.into_iter().map(User::from).collect())
    /// }
    /// ```
    pub fn query_method(&self) -> TokenStream {
        if !self.entity.has_filters() {
            return TokenStream::new();
        }

        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            soft_delete,
            ..
        } = self;

        let query_type = self.entity.ident_with("", "Query");
        let filter_fields = self.entity.filter_fields();

        let where_conditions = generate_where_conditions(&filter_fields, *soft_delete);
        let bindings = generate_query_bindings(&filter_fields);

        quote! {
            async fn query(&self, query: #query_type) -> Result<Vec<#entity_name>, Self::Error> {
                let mut conditions: Vec<String> = Vec::new();
                let mut param_idx: usize = 1;

                #where_conditions

                let where_clause = if conditions.is_empty() {
                    String::new()
                } else {
                    format!("WHERE {}", conditions.join(" AND "))
                };

                let limit_idx = param_idx;
                param_idx += 1;
                let offset_idx = param_idx;

                let sql = format!(
                    "SELECT {} FROM {} {} ORDER BY {} DESC LIMIT ${} OFFSET ${}",
                    #columns_str, #table, where_clause, stringify!(#id_name), limit_idx, offset_idx
                );

                let mut q = sqlx::query_as::<_, #row_name>(&sql);
                #bindings
                q = q.bind(query.limit.unwrap_or(100)).bind(query.offset.unwrap_or(0));

                let rows = q.fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}
