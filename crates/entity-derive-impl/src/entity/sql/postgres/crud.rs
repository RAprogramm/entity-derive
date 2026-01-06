// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! CRUD method generators for PostgreSQL.
//!
//! This module generates the core repository methods:
//!
//! | Method | SQL Operation |
//! |--------|---------------|
//! | [`create`](Context::create_method) | `INSERT INTO ... VALUES ... RETURNING ...` |
//! | [`find_by_id`](Context::find_by_id_method) | `SELECT ... WHERE id = $1` |
//! | [`update`](Context::update_method) | `UPDATE ... SET ... WHERE id = $n` |
//! | [`delete`](Context::delete_method) | `DELETE FROM ... WHERE id = $1` |
//! | [`list`](Context::list_method) | `SELECT ... ORDER BY ... LIMIT ... OFFSET ...` |
//!
//! # RETURNING Modes
//!
//! The `create` and `update` methods respect the entity's `returning`
//! configuration:
//!
//! | Mode | Behavior |
//! |------|----------|
//! | `Full` | Uses `RETURNING *` to fetch all columns |
//! | `Id` | Uses `RETURNING id` for minimal overhead |
//! | `None` | No RETURNING clause (fire-and-forget) |
//! | `Custom` | Returns specified columns |

use proc_macro2::TokenStream;
use quote::quote;

use super::{
    context::Context,
    helpers::{insert_bindings, update_bindings}
};
use crate::entity::parse::ReturningMode;

impl Context<'_> {
    /// Generate the `create` method implementation.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// INSERT INTO schema.table (col1, col2, ...)
    /// VALUES ($1, $2, ...)
    /// RETURNING *  -- depends on returning mode
    /// ```
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` if entity has no create fields.
    pub fn create_method(&self) -> TokenStream {
        if self.entity.create_fields().is_empty() {
            return TokenStream::new();
        }

        let Self {
            entity_name,
            row_name,
            insertable_name,
            create_dto,
            table,
            columns_str,
            placeholders_str,
            entity,
            returning,
            ..
        } = self;
        let bindings = insert_bindings(entity.all_fields());

        match returning {
            ReturningMode::Full => {
                quote! {
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        let row: #row_name = sqlx::query_as(
                            concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                        )
                            #(#bindings)*
                            .fetch_one(self).await?;
                        Ok(#entity_name::from(row))
                    }
                }
            }
            ReturningMode::Id => {
                let id_name = self.id_name;
                quote! {
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING ", stringify!(#id_name)))
                            #(#bindings)*
                            .execute(self).await?;
                        Ok(entity)
                    }
                }
            }
            ReturningMode::None => {
                quote! {
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ")"))
                            #(#bindings)*
                            .execute(self).await?;
                        Ok(entity)
                    }
                }
            }
            ReturningMode::Custom(columns) => {
                let returning_cols = columns.join(", ");
                quote! {
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(&format!("INSERT INTO {} ({}) VALUES ({}) RETURNING {}", #table, #columns_str, #placeholders_str, #returning_cols))
                            #(#bindings)*
                            .execute(self).await?;
                        Ok(entity)
                    }
                }
            }
        }
    }

    /// Generate the `find_by_id` method implementation.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT col1, col2, ... FROM schema.table
    /// WHERE id = $1
    /// AND deleted_at IS NULL  -- if soft_delete enabled
    /// ```
    pub fn find_by_id_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            id_type,
            dialect,
            soft_delete,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);
        let deleted_filter = if *soft_delete {
            " AND deleted_at IS NULL"
        } else {
            ""
        };

        quote! {
            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} WHERE {} = {}{}", #columns_str, #table, stringify!(#id_name), #placeholder, #deleted_filter)
                ).bind(&id).fetch_optional(self).await?;
                Ok(row.map(#entity_name::from))
            }
        }
    }

    /// Generate the `update` method implementation.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// UPDATE schema.table
    /// SET col1 = $1, col2 = $2, ...
    /// WHERE id = $n
    /// RETURNING *  -- depends on returning mode
    /// ```
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` if entity has no update fields.
    pub fn update_method(&self) -> TokenStream {
        let update_fields = self.entity.update_fields();
        if update_fields.is_empty() {
            return TokenStream::new();
        }

        let Self {
            entity_name,
            row_name,
            update_dto,
            table,
            id_name,
            id_type,
            dialect,
            trait_name,
            returning,
            ..
        } = self;

        let field_names: Vec<String> = update_fields.iter().map(|f| f.name_str()).collect();
        let field_refs: Vec<&str> = field_names.iter().map(String::as_str).collect();
        let set_clause = dialect.set_clause(&field_refs);
        let where_placeholder = dialect.placeholder(update_fields.len() + 1);
        let bindings = update_bindings(&update_fields);

        match returning {
            ReturningMode::Full => {
                quote! {
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        let row: #row_name = sqlx::query_as(
                            &format!("UPDATE {} SET {} WHERE {} = {} RETURNING *", #table, #set_clause, stringify!(#id_name), #where_placeholder)
                        )
                            #(#bindings)*
                            .bind(&id)
                            .fetch_one(self).await?;
                        Ok(#entity_name::from(row))
                    }
                }
            }
            ReturningMode::Id | ReturningMode::None => {
                quote! {
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        sqlx::query(&format!("UPDATE {} SET {} WHERE {} = {}", #table, #set_clause, stringify!(#id_name), #where_placeholder))
                            #(#bindings)*
                            .bind(&id)
                            .execute(self).await?;
                        <Self as #trait_name>::find_by_id(self, id).await?.ok_or_else(|| sqlx::Error::RowNotFound.into())
                    }
                }
            }
            ReturningMode::Custom(columns) => {
                let returning_cols = columns.join(", ");
                quote! {
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        sqlx::query(&format!("UPDATE {} SET {} WHERE {} = {} RETURNING {}", #table, #set_clause, stringify!(#id_name), #where_placeholder, #returning_cols))
                            #(#bindings)*
                            .bind(&id)
                            .execute(self).await?;
                        <Self as #trait_name>::find_by_id(self, id).await?.ok_or_else(|| sqlx::Error::RowNotFound.into())
                    }
                }
            }
        }
    }

    /// Generate the `delete` method implementation.
    ///
    /// # SQL Pattern
    ///
    /// Normal delete:
    /// ```sql
    /// DELETE FROM schema.table WHERE id = $1
    /// ```
    ///
    /// Soft delete:
    /// ```sql
    /// UPDATE schema.table SET deleted_at = NOW()
    /// WHERE id = $1 AND deleted_at IS NULL
    /// ```
    pub fn delete_method(&self) -> TokenStream {
        let Self {
            table,
            id_name,
            id_type,
            dialect,
            soft_delete,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        if *soft_delete {
            quote! {
                async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                    let result = sqlx::query(&format!(
                        "UPDATE {} SET deleted_at = NOW() WHERE {} = {} AND deleted_at IS NULL",
                        #table, stringify!(#id_name), #placeholder
                    )).bind(&id).execute(self).await?;
                    Ok(result.rows_affected() > 0)
                }
            }
        } else {
            quote! {
                async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                    let result = sqlx::query(&format!("DELETE FROM {} WHERE {} = {}", #table, stringify!(#id_name), #placeholder))
                        .bind(&id).execute(self).await?;
                    Ok(result.rows_affected() > 0)
                }
            }
        }
    }

    /// Generate the `list` method implementation.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT col1, col2, ... FROM schema.table
    /// WHERE deleted_at IS NULL  -- if soft_delete enabled
    /// ORDER BY id DESC
    /// LIMIT $1 OFFSET $2
    /// ```
    pub fn list_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            dialect,
            soft_delete,
            ..
        } = self;
        let limit_placeholder = dialect.placeholder(1);
        let offset_placeholder = dialect.placeholder(2);
        let where_clause = if *soft_delete {
            "WHERE deleted_at IS NULL "
        } else {
            ""
        };

        quote! {
            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} {}ORDER BY {} DESC LIMIT {} OFFSET {}",
                        #columns_str, #table, #where_clause, stringify!(#id_name), #limit_placeholder, #offset_placeholder)
                ).bind(limit).bind(offset).fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}
