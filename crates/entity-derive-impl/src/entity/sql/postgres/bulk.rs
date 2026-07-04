// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Bulk operation generators for `PostgreSQL`.
//!
//! | Method | SQL |
//! |--------|-----|
//! | `find_by_ids` | `SELECT ... WHERE id = ANY($1)` |
//! | `create_many` | per-row `INSERT ... RETURNING` inside one transaction |
//! | `delete_many` | `DELETE`/soft-`UPDATE` with `id = ANY($1) RETURNING id` |
//!
//! All bulk writes are atomic: a failing row rolls back the whole
//! batch. Events, NOTIFY and the transactional outbox are emitted per
//! affected row, matching the single-row methods.

use proc_macro2::TokenStream;
use quote::quote;

use super::{context::Context, crud::tx_wrapping, helpers::insert_bindings};
use crate::{entity::parse::ReturningMode, utils::tracing::instrument};

impl Context<'_> {
    /// Generate the `find_by_ids` method implementation.
    pub fn find_by_ids_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            id_type,
            soft_delete,
            ..
        } = self;

        let deleted_filter = if *soft_delete {
            " AND deleted_at IS NULL"
        } else {
            ""
        };
        let sql =
            format!("SELECT {columns_str} FROM {table} WHERE {id_name} = ANY($1){deleted_filter}");
        let span = instrument(&entity_name.to_string(), "find_by_ids");

        quote! {
            #span
            async fn find_by_ids(&self, ids: Vec<#id_type>) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(#sql)
                    .bind(&ids)
                    .fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }

    /// Generate the `create_many` method implementation.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` if entity has no create fields.
    pub fn create_many_method(&self) -> TokenStream {
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
        let span = instrument(&entity_name.to_string(), "create_many");
        let outbox_created = self.outbox_created();
        let notify = self.notify_created();
        let constraint_map_err = self.constraint_map_err();

        let insert_row = if matches!(returning, ReturningMode::Full) {
            quote! {
                let row: #row_name = sqlx::query_as(
                    concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                )
                    #(#bindings)*
                    .fetch_one(&mut *tx).await #constraint_map_err?;
                let entity = #entity_name::from(row);
            }
        } else {
            quote! {
                sqlx::query(concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ")"))
                    #(#bindings)*
                    .execute(&mut *tx).await #constraint_map_err?;
            }
        };

        quote! {
            #span
            async fn create_many(
                &self,
                dtos: Vec<#create_dto>,
            ) -> Result<Vec<#entity_name>, Self::Error> {
                let mut tx = self.begin().await?;
                let mut created = Vec::with_capacity(dtos.len());
                for dto in dtos {
                    let entity = #entity_name::from(dto);
                    let insertable = #insertable_name::from(&entity);
                    #insert_row
                    #outbox_created
                    #notify
                    created.push(entity);
                }
                tx.commit().await?;
                Ok(created)
            }
        }
    }

    /// Generate the `delete_many` method implementation.
    ///
    /// Soft-delete aware; per-row delete events are emitted inside the
    /// same transaction when events delivery is enabled.
    pub fn delete_many_method(&self) -> TokenStream {
        let Self {
            entity_name,
            table,
            id_name,
            id_type,
            soft_delete,
            streams,
            ..
        } = self;

        let sql = if *soft_delete {
            format!(
                "UPDATE {table} SET deleted_at = NOW() \
                 WHERE {id_name} = ANY($1) AND deleted_at IS NULL RETURNING {id_name}"
            )
        } else {
            format!("DELETE FROM {table} WHERE {id_name} = ANY($1) RETURNING {id_name}")
        };
        let span = instrument(&entity_name.to_string(), "delete_many");

        let emit_events = *streams || self.outbox;
        if !emit_events {
            return quote! {
                #span
                async fn delete_many(&self, ids: Vec<#id_type>) -> Result<u64, Self::Error> {
                    let deleted: Vec<#id_type> = sqlx::query_scalar(#sql)
                        .bind(&ids)
                        .fetch_all(self).await?;
                    Ok(deleted.len() as u64)
                }
            };
        }

        let (tx_open, tx_close, _executor) = tx_wrapping(true);
        let outbox_deleted = self.outbox_deleted();
        let notify = if *soft_delete {
            self.notify_soft_deleted()
        } else {
            self.notify_hard_deleted()
        };

        quote! {
            #span
            async fn delete_many(&self, ids: Vec<#id_type>) -> Result<u64, Self::Error> {
                #tx_open
                let deleted: Vec<#id_type> = sqlx::query_scalar(#sql)
                    .bind(&ids)
                    .fetch_all(&mut *tx).await?;
                let count = deleted.len() as u64;
                for id in deleted {
                    #outbox_deleted
                    #notify
                }
                #tx_close
                Ok(count)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::super::context::Context;
    use crate::entity::parse::EntityDef;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    fn post_entity(extra: proc_macro2::TokenStream) -> EntityDef {
        parse_entity(quote! {
            #[entity(table = "posts" #extra)]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub title: String,
            }
        })
    }

    #[test]
    fn find_by_ids_uses_any() {
        let entity = post_entity(quote!());
        let code = Context::new(&entity).find_by_ids_method().to_string();
        assert!(code.contains("id = ANY($1)"));
    }

    #[test]
    fn find_by_ids_respects_soft_delete() {
        let entity = parse_entity(quote! {
            #[entity(table = "posts", soft_delete)]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub title: String,
                pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
            }
        });
        let code = Context::new(&entity).find_by_ids_method().to_string();
        assert!(code.contains("AND deleted_at IS NULL"));
    }

    #[test]
    fn create_many_wraps_in_transaction() {
        let entity = post_entity(quote!());
        let code = Context::new(&entity).create_many_method().to_string();
        assert!(code.contains("self . begin ()"));
        assert!(code.contains("RETURNING *"));
        assert!(code.contains("tx . commit ()"));
    }

    #[test]
    fn create_many_absent_without_create_fields() {
        let entity = parse_entity(quote! {
            #[entity(table = "counters")]
            pub struct Counter {
                #[id]
                pub id: uuid::Uuid,
                #[field(response)]
                #[auto]
                pub value: i64,
            }
        });
        assert!(Context::new(&entity).create_many_method().is_empty());
    }

    #[test]
    fn delete_many_plain_counts_without_transaction() {
        let entity = post_entity(quote!());
        let code = Context::new(&entity).delete_many_method().to_string();
        assert!(code.contains("DELETE FROM posts WHERE id = ANY($1) RETURNING id"));
        assert!(!code.contains("begin"));
    }

    #[test]
    fn delete_many_soft_delete_updates() {
        let entity = parse_entity(quote! {
            #[entity(table = "posts", soft_delete)]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub title: String,
                pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
            }
        });
        let code = Context::new(&entity).delete_many_method().to_string();
        assert!(code.contains("SET deleted_at = NOW()"));
        assert!(code.contains("AND deleted_at IS NULL RETURNING id"));
    }

    #[test]
    fn delete_many_with_outbox_emits_per_row() {
        let entity = post_entity(quote!(, events(outbox)));
        let code = Context::new(&entity).delete_many_method().to_string();
        assert!(code.contains("begin"));
        assert!(code.contains("for id in deleted"));
        assert!(code.contains("entity_outbox"));
    }
}
