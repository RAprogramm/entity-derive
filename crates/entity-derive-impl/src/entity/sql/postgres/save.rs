// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Save method generator for aggregate root entities.
//!
//! Generates the `save` method implementation for the repository trait.
//! This method performs a transactional insert of the parent entity.
//!
//! # Generated Code
//!
//! For an `Order` aggregate root:
//!
//! ```rust,ignore
//! async fn save(&self, new: NewOrder) -> Result<Order, Self::Error> {
//!     let mut tx = self.begin().await?;
//!
//!     let mut entity: Order = new.into();
//!     let insertable = InsertableOrder::from(&entity);
//!     let row: OrderRow = sqlx::query_as(
//!         "INSERT INTO orders (...) VALUES (...) RETURNING *"
//!     )
//!         .bind(insertable.buyer_id)
//!         .bind(insertable.seller_id)
//!         .fetch_one(&mut *tx).await?;
//!     entity = Order::from(row);
//!
//!     tx.commit().await?;
//!     Ok(entity)
//! }
//! ```
//!
//! # Transaction Semantics
//!
//! The method uses sqlx transactions:
//!
//! 1. `BEGIN` via `self.begin().await`
//! 2. Insert parent with `RETURNING *`
//! 3. `COMMIT` — if any step fails, the transaction is rolled back

use proc_macro2::TokenStream;
use quote::quote;

use super::context::Context;
use crate::{entity::parse::SqlLevel, utils::tracing::instrument};

impl Context<'_> {
    /// Generate the `save` method implementation for aggregate roots.
    ///
    /// Creates a transactional insert for the parent entity.
    /// Returns just the method body, to be included in the main
    /// repository trait impl block.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` if `sql != "full"` or `aggregate_root` is not
    /// enabled.
    pub fn save_method(&self) -> TokenStream {
        if self.entity.sql != SqlLevel::Full {
            return TokenStream::new();
        }

        if !self.entity.is_aggregate_root() {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let new_name = self.entity.ident_with("New", "");
        let row_name = &self.row_name;
        let insertable_name = &self.insertable_name;
        let table = &self.table;
        let insert_columns_str = &self.insert_columns_str;
        let placeholders_str = &self.placeholders_str;
        let bindings = super::helpers::insert_bindings(self.entity.all_fields());
        let error_type = self.entity.error_type();

        let span = instrument(&entity_name.to_string(), "save");

        // `notify_created` returns an empty TokenStream when streams are off,
        // so the same generator covers both streams-enabled and plain
        // aggregate roots. When streams ARE on, the splice runs against the
        // same transaction (`&mut *tx`) that wraps the INSERT, so Postgres
        // only broadcasts the `Created` event on commit and discards it on
        // rollback — atomic with the row write. See #133.
        let notify = self.notify_created();

        quote! {
            #span
            async fn save(&self, new: #new_name) -> Result<#entity_name, #error_type> {
                let mut tx = self.begin().await?;

                let mut entity: #entity_name = new.into();
                let insertable = #insertable_name::from(&entity);
                let row: #row_name = sqlx::query_as(
                    concat!("INSERT INTO ", #table, " (", #insert_columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                )
                    #(#bindings)*
                    .fetch_one(&mut *tx).await?;
                entity = #entity_name::from(row);

                #notify

                tx.commit().await?;
                Ok(entity)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;
    use crate::entity::parse::EntityDef;

    fn ctx_for(input: syn::DeriveInput) -> Context<'static> {
        // Leak the parsed entity so the borrowed-against `Context` outlives
        // the test scope. Acceptable in test code; saves shuffling lifetimes.
        let entity: &'static EntityDef = Box::leak(Box::new(
            EntityDef::from_derive_input(&input).expect("parse ok")
        ));
        Context::new(entity)
    }

    #[test]
    #[cfg(feature = "streams")]
    fn save_emits_pg_notify_when_streams_enabled() {
        let ctx = ctx_for(parse_quote! {
            #[entity(table = "users", aggregate_root, streams)]
            pub struct User {
                #[id]
                pub id: ::uuid::Uuid,
                #[field(create, update, response)]
                pub email: String
            }
        });
        let tokens = ctx.save_method().to_string();
        assert!(
            tokens.contains("pg_notify"),
            "streams + aggregate_root must splice pg_notify into save(), got: {tokens}"
        );
        // The notify must run on the transaction handle so it commits
        // atomically with the INSERT, not after.
        assert!(
            tokens.contains("& mut * tx"),
            "pg_notify must execute on `&mut *tx`, got: {tokens}"
        );
    }

    #[test]
    fn save_omits_pg_notify_when_streams_disabled() {
        let ctx = ctx_for(parse_quote! {
            #[entity(table = "users", aggregate_root)]
            pub struct User {
                #[id]
                pub id: ::uuid::Uuid,
                #[field(create, update, response)]
                pub email: String
            }
        });
        let tokens = ctx.save_method().to_string();
        assert!(
            !tokens.contains("pg_notify"),
            "non-streams aggregate root must NOT emit pg_notify (perf regression guard), got: {tokens}"
        );
    }

    #[test]
    fn save_is_empty_for_non_aggregate_root() {
        let ctx = ctx_for(parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: ::uuid::Uuid
            }
        });
        let tokens = ctx.save_method();
        assert!(
            tokens.is_empty(),
            "save() must not be generated unless aggregate_root is on, got: {tokens}"
        );
    }
}
