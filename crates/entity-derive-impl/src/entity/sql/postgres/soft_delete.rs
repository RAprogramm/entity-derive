// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Soft delete method generators for PostgreSQL.
//!
//! When an entity has `#[entity(soft_delete)]`, these additional methods
//! are generated to handle soft-deleted records:
//!
//! | Method | Description |
//! |--------|-------------|
//! | `hard_delete` | Permanently removes the record |
//! | `restore` | Undeletes by setting `deleted_at = NULL` |
//! | `find_by_id_with_deleted` | Finds including soft-deleted records |
//! | `list_with_deleted` | Lists including soft-deleted records |
//!
//! # Soft Delete Pattern
//!
//! Instead of `DELETE FROM table`, soft delete uses:
//! ```sql
//! UPDATE table SET deleted_at = NOW() WHERE id = $1
//! ```
//!
//! Regular `find_by_id` and `list` automatically filter out deleted records.

use proc_macro2::TokenStream;
use quote::quote;

use super::context::Context;

impl Context<'_> {
    /// Generate all soft delete methods.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` if soft delete is not enabled.
    pub fn soft_delete_methods(&self) -> TokenStream {
        if !self.soft_delete {
            return TokenStream::new();
        }

        let hard_delete = self.hard_delete_method();
        let restore = self.restore_method();
        let find_with_deleted = self.find_by_id_with_deleted_method();
        let list_with_deleted = self.list_with_deleted_method();

        quote! {
            #hard_delete
            #restore
            #find_with_deleted
            #list_with_deleted
        }
    }

    /// Generate the `hard_delete` method.
    ///
    /// Permanently removes the record from the database.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// DELETE FROM schema.table WHERE id = $1
    /// ```
    fn hard_delete_method(&self) -> TokenStream {
        let Self {
            table,
            id_name,
            id_type,
            dialect,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        quote! {
            async fn hard_delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                let result = sqlx::query(&format!(
                    "DELETE FROM {} WHERE {} = {}",
                    #table, stringify!(#id_name), #placeholder
                )).bind(&id).execute(self).await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    /// Generate the `restore` method.
    ///
    /// Undeletes a soft-deleted record by setting `deleted_at = NULL`.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// UPDATE schema.table SET deleted_at = NULL
    /// WHERE id = $1 AND deleted_at IS NOT NULL
    /// ```
    fn restore_method(&self) -> TokenStream {
        let Self {
            table,
            id_name,
            id_type,
            dialect,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        quote! {
            async fn restore(&self, id: #id_type) -> Result<bool, Self::Error> {
                let result = sqlx::query(&format!(
                    "UPDATE {} SET deleted_at = NULL WHERE {} = {} AND deleted_at IS NOT NULL",
                    #table, stringify!(#id_name), #placeholder
                )).bind(&id).execute(self).await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    /// Generate the `find_by_id_with_deleted` method.
    ///
    /// Finds a record by ID without filtering out soft-deleted records.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT ... FROM schema.table WHERE id = $1
    /// ```
    ///
    /// Note: No `AND deleted_at IS NULL` filter.
    fn find_by_id_with_deleted_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            id_type,
            dialect,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        quote! {
            async fn find_by_id_with_deleted(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} WHERE {} = {}", #columns_str, #table, stringify!(#id_name), #placeholder)
                ).bind(&id).fetch_optional(self).await?;
                Ok(row.map(#entity_name::from))
            }
        }
    }

    /// Generate the `list_with_deleted` method.
    ///
    /// Lists records without filtering out soft-deleted records.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT ... FROM schema.table
    /// ORDER BY id DESC
    /// LIMIT $1 OFFSET $2
    /// ```
    ///
    /// Note: No `WHERE deleted_at IS NULL` filter.
    fn list_with_deleted_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            dialect,
            ..
        } = self;
        let limit_placeholder = dialect.placeholder(1);
        let offset_placeholder = dialect.placeholder(2);

        quote! {
            async fn list_with_deleted(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} ORDER BY {} DESC LIMIT {} OFFSET {}",
                        #columns_str, #table, stringify!(#id_name), #limit_placeholder, #offset_placeholder)
                ).bind(limit).bind(offset).fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}
