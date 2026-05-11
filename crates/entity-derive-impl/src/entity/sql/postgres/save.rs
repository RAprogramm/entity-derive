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
use crate::entity::parse::SqlLevel;

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
        let columns_str = &self.columns_str;
        let placeholders_str = &self.placeholders_str;
        let bindings = super::helpers::insert_bindings(self.entity.all_fields());
        let error_type = self.entity.error_type();

        quote! {
            async fn save(&self, new: #new_name) -> Result<#entity_name, #error_type> {
                let mut tx = self.begin().await?;

                let mut entity: #entity_name = new.into();
                let insertable = #insertable_name::from(&entity);
                let row: #row_name = sqlx::query_as(
                    concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                )
                    #(#bindings)*
                    .fetch_one(&mut *tx).await?;
                entity = #entity_name::from(row);

                tx.commit().await?;
                Ok(entity)
            }
        }
    }
}
