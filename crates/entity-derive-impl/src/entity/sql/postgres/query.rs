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

    /// Generate the `stream_filtered` method implementation.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` if entity has no streams or filter fields.
    pub fn stream_filtered_method(&self) -> TokenStream {
        if !self.streams || !self.entity.has_filters() {
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

        let filter_type = self.entity.ident_with("", "Filter");
        let filter_fields = self.entity.filter_fields();

        let where_conditions = generate_where_conditions(&filter_fields, *soft_delete);
        let bindings = generate_query_bindings(&filter_fields);

        // For now, generate a simple implementation that fetches all and converts to
        // stream True streaming would require more complex lifetime handling
        quote! {
            async fn stream_filtered(
                &self,
                filter: #filter_type,
            ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<#entity_name, Self::Error>> + Send + '_>>, Self::Error> {
                use futures::StreamExt;

                let mut conditions: Vec<String> = Vec::new();
                let mut param_idx: usize = 1;
                // Rename filter to query for binding code compatibility
                let query = filter;

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
                q = q.bind(query.limit.unwrap_or(10000)).bind(query.offset.unwrap_or(0));

                // Fetch all results and convert to stream for simpler lifetime handling
                let rows = q.fetch_all(self).await?;
                let entities: Vec<#entity_name> = rows.into_iter().map(#entity_name::from).collect();
                let stream = futures::stream::iter(entities.into_iter().map(Ok));

                Ok(Box::pin(stream))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::parse::EntityDef;

    #[test]
    fn query_method_no_filters_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let ctx = Context::new(&entity);
        let method = ctx.query_method();
        assert!(method.is_empty());
    }

    #[test]
    fn query_method_with_filter() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[filter]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let ctx = Context::new(&entity);
        let method = ctx.query_method();
        let method_str = method.to_string();
        assert!(method_str.contains("async fn query"));
        assert!(method_str.contains("UserQuery"));
        assert!(method_str.contains("conditions"));
        assert!(method_str.contains("where_clause"));
    }

    #[test]
    fn query_method_with_soft_delete() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", soft_delete)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[filter]
                pub name: String,
                #[field(response)]
                #[auto]
                pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let ctx = Context::new(&entity);
        let method = ctx.query_method();
        let method_str = method.to_string();
        assert!(method_str.contains("deleted_at"));
    }

    #[test]
    fn stream_filtered_no_streams_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[filter]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let ctx = Context::new(&entity);
        let method = ctx.stream_filtered_method();
        assert!(method.is_empty());
    }

    #[test]
    fn stream_filtered_no_filters_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", streams)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let ctx = Context::new(&entity);
        let method = ctx.stream_filtered_method();
        assert!(method.is_empty());
    }

    #[test]
    fn stream_filtered_with_streams_and_filters() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", streams)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[filter]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let ctx = Context::new(&entity);
        let method = ctx.stream_filtered_method();
        let method_str = method.to_string();
        assert!(method_str.contains("stream_filtered"));
        assert!(method_str.contains("UserFilter"));
        assert!(method_str.contains("futures"));
    }
}
