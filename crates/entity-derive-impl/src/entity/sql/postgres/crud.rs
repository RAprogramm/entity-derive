// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! CRUD method generators for `PostgreSQL`.
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
use crate::{entity::parse::ReturningMode, utils::tracing::instrument};

/// Return the `(open, close, executor)` token fragments that bracket
/// streams-aware DML.
///
/// - When `streams` is `true`, the caller wraps its work in a transaction so
///   the DML and the subsequent `pg_notify` participate in one atomic unit.
///   `open` opens the transaction, `close` commits it, and `executor` is `&mut
///   *tx` (the right argument for `.execute` / `.fetch_one` on a transaction
///   handle).
/// - When `streams` is `false`, the wrapping fragments are empty and the
///   executor is `self`, keeping the generated method a single SQL round-trip
///   with no behavior change vs. earlier releases.
pub(super) fn tx_wrapping(streams: bool) -> (TokenStream, TokenStream, TokenStream) {
    if streams {
        (
            quote! { let mut tx = self.begin().await?; },
            quote! { tx.commit().await?; },
            quote! { &mut *tx }
        )
    } else {
        (TokenStream::new(), TokenStream::new(), quote! { self })
    }
}

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
            streams,
            ..
        } = self;
        let bindings = insert_bindings(entity.all_fields());

        let span = instrument(&entity_name.to_string(), "create");
        // When `streams` is on, the DML and the `pg_notify` must commit as
        // one unit: Postgres only broadcasts `NOTIFY` on commit and discards
        // it on rollback, so wrapping both in a transaction eliminates the
        // crash-between-commit-and-notify window. Non-streams entities keep
        // the single-statement fast path.
        let (tx_open, tx_close, executor) = tx_wrapping(*streams || self.outbox);
        let outbox_created = self.outbox_created();
        let notify = self.notify_created();

        match returning {
            ReturningMode::Full => {
                quote! {
                    #span
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        #tx_open
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        let row: #row_name = sqlx::query_as(
                            concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                        )
                            #(#bindings)*
                            .fetch_one(#executor).await?;
                        let entity = #entity_name::from(row);
                        #outbox_created
                        #notify
                        #tx_close
                        Ok(entity)
                    }
                }
            }
            ReturningMode::Id => {
                let id_name = self.id_name;
                quote! {
                    #span
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        #tx_open
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING ", stringify!(#id_name)))
                            #(#bindings)*
                            .execute(#executor).await?;
                        #outbox_created
                        #notify
                        #tx_close
                        Ok(entity)
                    }
                }
            }
            ReturningMode::None => {
                quote! {
                    #span
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        #tx_open
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ")"))
                            #(#bindings)*
                            .execute(#executor).await?;
                        #outbox_created
                        #notify
                        #tx_close
                        Ok(entity)
                    }
                }
            }
            ReturningMode::Custom(columns) => {
                let returning_cols = columns.join(", ");
                quote! {
                    #span
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        #tx_open
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(::sqlx::AssertSqlSafe(format!("INSERT INTO {} ({}) VALUES ({}) RETURNING {}", #table, #columns_str, #placeholders_str, #returning_cols)))
                            #(#bindings)*
                            .execute(#executor).await?;
                        #outbox_created
                        #notify
                        #tx_close
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

        let span = instrument(&entity_name.to_string(), "find_by_id");

        quote! {
            #span
            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    ::sqlx::AssertSqlSafe(format!("SELECT {} FROM {} WHERE {} = {}{}", #columns_str, #table, stringify!(#id_name), #placeholder, #deleted_filter))
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
            streams,
            ..
        } = self;

        let field_names: Vec<String> = update_fields.iter().map(|f| f.name_str()).collect();
        let field_refs: Vec<&str> = field_names.iter().map(String::as_str).collect();
        let set_clause = dialect.set_clause(&field_refs);
        let where_placeholder = dialect.placeholder(update_fields.len() + 1);
        let bindings = update_bindings(&update_fields);

        let span = instrument(&entity_name.to_string(), "update");

        // Streams-on path: SELECT FOR UPDATE → UPDATE RETURNING * → notify,
        // all in one transaction. Always fetches the full row (regardless
        // of `returning` config) because the Updated event payload requires
        // it; a streams entity that asked for a narrower `RETURNING` clause
        // would have to re-fetch separately anyway, defeating the optimization.
        if *streams || self.outbox {
            let fetch_old = self.fetch_old_for_update();
            let outbox_updated = self.outbox_updated();
            let notify = self.notify_updated();
            return quote! {
                #span
                async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                    let mut tx = self.begin().await?;
                    #fetch_old
                    let row: #row_name = sqlx::query_as(
                        ::sqlx::AssertSqlSafe(format!("UPDATE {} SET {} WHERE {} = {} RETURNING *", #table, #set_clause, stringify!(#id_name), #where_placeholder))
                    )
                        #(#bindings)*
                        .bind(&id)
                        .fetch_one(&mut *tx).await?;
                    let entity = #entity_name::from(row);
                    #outbox_updated
                    #notify
                    tx.commit().await?;
                    Ok(entity)
                }
            };
        }

        // Non-streams path: keep the historical single-statement variants,
        // honoring the entity's `returning` configuration. Notify is empty
        // here so we don't pay for a transaction we don't need.
        match returning {
            ReturningMode::Full => {
                quote! {
                    #span
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        let row: #row_name = sqlx::query_as(
                            ::sqlx::AssertSqlSafe(format!("UPDATE {} SET {} WHERE {} = {} RETURNING *", #table, #set_clause, stringify!(#id_name), #where_placeholder))
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
                    #span
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        sqlx::query(::sqlx::AssertSqlSafe(format!("UPDATE {} SET {} WHERE {} = {}", #table, #set_clause, stringify!(#id_name), #where_placeholder)))
                            #(#bindings)*
                            .bind(&id)
                            .execute(self).await?;
                        <Self as #trait_name>::find_by_id(self, id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
                    }
                }
            }
            ReturningMode::Custom(columns) => {
                let returning_cols = columns.join(", ");
                quote! {
                    #span
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        sqlx::query(::sqlx::AssertSqlSafe(format!("UPDATE {} SET {} WHERE {} = {} RETURNING {}", #table, #set_clause, stringify!(#id_name), #where_placeholder, #returning_cols)))
                            #(#bindings)*
                            .bind(&id)
                            .execute(self).await?;
                        <Self as #trait_name>::find_by_id(self, id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
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
            entity_name,
            table,
            id_name,
            id_type,
            dialect,
            soft_delete,
            streams,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);
        // When streams are on, wrap the DML + notify in one transaction so
        // the event commits atomically with the deletion. Single SQL
        // statement otherwise — no perf regression for non-streams entities.
        let (tx_open, tx_close, executor) = tx_wrapping(*streams || self.outbox);
        let outbox_deleted = self.outbox_deleted();

        if *soft_delete {
            let notify = self.notify_soft_deleted();
            let span = instrument(&entity_name.to_string(), "soft_delete");
            quote! {
                #span
                async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                    #tx_open
                    let result = sqlx::query(::sqlx::AssertSqlSafe(format!(
                        "UPDATE {} SET deleted_at = NOW() WHERE {} = {} AND deleted_at IS NULL",
                        #table, stringify!(#id_name), #placeholder
                    ))).bind(&id).execute(#executor).await?;
                    let deleted = result.rows_affected() > 0;
                    if deleted {
                        #outbox_deleted
                        #notify
                    }
                    #tx_close
                    Ok(deleted)
                }
            }
        } else {
            let notify = self.notify_hard_deleted();
            let span = instrument(&entity_name.to_string(), "delete");
            quote! {
                #span
                async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                    #tx_open
                    let result = sqlx::query(::sqlx::AssertSqlSafe(format!("DELETE FROM {} WHERE {} = {}", #table, stringify!(#id_name), #placeholder)))
                        .bind(&id).execute(#executor).await?;
                    let deleted = result.rows_affected() > 0;
                    if deleted {
                        #outbox_deleted
                        #notify
                    }
                    #tx_close
                    Ok(deleted)
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

        let span = instrument(&entity_name.to_string(), "list");

        quote! {
            #span
            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(
                    ::sqlx::AssertSqlSafe(format!("SELECT {} FROM {} {}ORDER BY {} DESC LIMIT {} OFFSET {}",
                        #columns_str, #table, #where_clause, stringify!(#id_name), #limit_placeholder, #offset_placeholder))
                ).bind(limit).bind(offset).fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}
