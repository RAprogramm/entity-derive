// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Participant-scope method generators for `PostgreSQL`.
//!
//! Generated from entity-level `#[scope(...)]` declarations:
//!
//! | Declaration | SQL |
//! |-------------|-----|
//! | `#[scope(involving: a \| b)]` | `SELECT ... WHERE (a = $1 OR b = $1) ...` |
//! | `#[scope(x: a \| b, within = p)]` | `SELECT ... WHERE p = $1 AND (a = $2 OR b = $2) ...` |
//!
//! One value is bound once and referenced by every branch of the OR
//! group. Soft-delete-aware, ordered by id descending like `list`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;
use crate::{entity::parse::ScopeDef, utils::tracing::instrument};

impl Context<'_> {
    /// Generate every declared scope method.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` when the entity declares no scopes.
    pub fn scope_methods(&self) -> TokenStream {
        let methods: Vec<TokenStream> = self
            .entity
            .scopes
            .iter()
            .map(|scope| self.scope_method(scope))
            .collect();

        quote! { #(#methods)* }
    }

    /// Generate one `list_{name}` implementation.
    fn scope_method(&self, scope: &ScopeDef) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            soft_delete,
            ..
        } = self;

        let method_name = format_ident!("{}", scope.method_name());
        let value_type = self.scope_value_type(scope);
        let within = scope.within.as_ref().map(|column| {
            let field = self
                .entity
                .column_fields()
                .into_iter()
                .find(|f| &f.name_str() == column)
                .expect("scope columns are validated during parsing");
            (format_ident!("{column}"), field.ty().clone())
        });

        let (prefix, value_index) = within.as_ref().map_or_else(
            || (String::new(), 1),
            |(column, _)| (format!("{column} = $1 AND "), 2)
        );
        let group = scope
            .columns
            .iter()
            .map(|column| format!("{column} = ${value_index}"))
            .collect::<Vec<_>>()
            .join(" OR ");
        let deleted_filter = if *soft_delete {
            " AND deleted_at IS NULL"
        } else {
            ""
        };
        let limit_index = value_index + 1;
        let offset_index = value_index + 2;
        let sql = format!(
            "SELECT {columns_str} FROM {table} WHERE {prefix}({group}){deleted_filter} \
             ORDER BY {id_name} DESC LIMIT ${limit_index} OFFSET ${offset_index}"
        );

        let span = instrument(&entity_name.to_string(), &scope.method_name());
        let (within_param, within_bind) = within.map_or_else(
            || (TokenStream::new(), TokenStream::new()),
            |(name, ty)| (quote! { #name: #ty, }, quote! { .bind(&#name) })
        );

        quote! {
            #span
            async fn #method_name(
                &self,
                #within_param
                value: #value_type,
                limit: i64,
                offset: i64,
            ) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(#sql)
                    #within_bind
                    .bind(&value)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(self)
                    .await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }

    /// Type of the value bound against the OR group.
    ///
    /// Parsing guarantees the columns agree, so the first one decides.
    fn scope_value_type(&self, scope: &ScopeDef) -> syn::Type {
        let first = &scope.columns[0];
        self.entity
            .column_fields()
            .into_iter()
            .find(|f| &f.name_str() == first)
            .expect("scope columns are validated during parsing")
            .ty()
            .clone()
    }
}
