// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Transaction support code generation.
//!
//! Generates transaction repository adapters and builder extensions
//! for type-safe multi-entity transactions.
//!
//! # Generated Types
//!
//! For an entity `User` with `#[entity(transactions)]`:
//!
//! - `UserTransactionRepo<'t>` — Repository adapter for transaction context
//! - `with_users()` — Builder method on `Transaction<..., ()>`
//! - `users()` — Accessor method on `TransactionContext`
//!
//! # Example
//!
//! ```rust,ignore
//! Transaction::new(&pool)
//!     .with_users()
//!     .with_orders()
//!     .run(|mut ctx| async move {
//!         let user = ctx.users().find_by_id(id).await?;
//!         ctx.orders().create(order).await?;
//!         Ok(())
//!     })
//!     .await?;
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::EntityDef;
use crate::utils::marker;

/// Generate all transaction-related code for an entity.
///
/// Returns empty `TokenStream` if `transactions` is not enabled.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_transactions() {
        return TokenStream::new();
    }

    let repo_adapter = generate_repo_adapter(entity);
    let builder_ext = generate_builder_extension(entity);

    quote! {
        #repo_adapter
        #builder_ext
    }
}

/// Generate the transaction repository adapter struct.
///
/// Creates a struct that wraps a transaction reference and provides
/// repository methods that operate within the transaction.
fn generate_repo_adapter(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let repo_name = format_ident!("{}TransactionRepo", entity_name);
    let create_dto = entity.ident_with("Create", "Request");
    let update_dto = entity.ident_with("Update", "Request");
    let id_type = entity.id_field().ty();
    let marker = marker::generated();

    let create_method = if entity.create_fields().is_empty() {
        TokenStream::new()
    } else {
        quote! {
            /// Create a new entity within the transaction.
            pub async fn create(
                &mut self,
                dto: #create_dto
            ) -> Result<#entity_name, sqlx::Error> {
                // Implementation delegated to generated SQL
                todo!("Transaction create not yet implemented")
            }
        }
    };

    let update_method = if entity.update_fields().is_empty() {
        TokenStream::new()
    } else {
        quote! {
            /// Update an entity within the transaction.
            pub async fn update(
                &mut self,
                id: #id_type,
                dto: #update_dto
            ) -> Result<#entity_name, sqlx::Error> {
                todo!("Transaction update not yet implemented")
            }
        }
    };

    quote! {
        #marker
        /// Transaction repository adapter for #entity_name.
        ///
        /// Provides repository operations that execute within an active transaction.
        /// Created via `Transaction::new(&pool).with_{entities}()`.
        #vis struct #repo_name<'t> {
            tx: &'t mut sqlx::Transaction<'static, sqlx::Postgres>,
        }

        impl<'t> #repo_name<'t> {
            /// Create a new transaction repository adapter.
            #[doc(hidden)]
            pub fn new(tx: &'t mut sqlx::Transaction<'static, sqlx::Postgres>) -> Self {
                Self { tx }
            }

            #create_method

            /// Find an entity by ID within the transaction.
            pub async fn find_by_id(
                &mut self,
                id: #id_type
            ) -> Result<Option<#entity_name>, sqlx::Error> {
                todo!("Transaction find_by_id not yet implemented")
            }

            #update_method

            /// Delete an entity within the transaction.
            pub async fn delete(
                &mut self,
                id: #id_type
            ) -> Result<bool, sqlx::Error> {
                todo!("Transaction delete not yet implemented")
            }

            /// List entities within the transaction.
            pub async fn list(
                &mut self,
                limit: i64,
                offset: i64
            ) -> Result<Vec<#entity_name>, sqlx::Error> {
                todo!("Transaction list not yet implemented")
            }
        }
    }
}

/// Generate the builder extension trait.
///
/// Creates an extension trait that adds `with_{entity}()` method to `Transaction`.
fn generate_builder_extension(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let method_name = format_ident!("with_{}", entity_snake);
    let trait_name = format_ident!("TransactionWith{}", entity_name);
    let repo_name = format_ident!("{}TransactionRepo", entity_name);
    let marker = marker::generated();

    quote! {
        #marker
        /// Extension trait to add #entity_name to a transaction.
        #vis trait #trait_name<'p> {
            /// Add #entity_name repository to the transaction.
            fn #method_name(self) -> entity_core::transaction::Transaction<'p, sqlx::PgPool, #repo_name<'static>>;
        }

        impl<'p> #trait_name<'p> for entity_core::transaction::Transaction<'p, sqlx::PgPool, ()> {
            fn #method_name(self) -> entity_core::transaction::Transaction<'p, sqlx::PgPool, #repo_name<'static>> {
                self.with_repo()
            }
        }
    }
}
