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
//! - `with_users()` — Builder method on `Transaction` (fluent, chainable)
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

use super::{parse::EntityDef, sql::postgres::Context};
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
    let context_ext = generate_context_extension(entity);

    quote! {
        #repo_adapter
        #builder_ext
        #context_ext
    }
}

/// Generate the transaction repository adapter struct.
///
/// Creates a struct that wraps a transaction reference and provides
/// repository methods that operate within the transaction.
fn generate_repo_adapter(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let ctx = Context::new(entity);
    let entity_name = ctx.entity_name;
    let row_name = &ctx.row_name;
    let insertable_name = &ctx.insertable_name;
    let create_dto = &ctx.create_dto;
    let update_dto = &ctx.update_dto;
    let table = &ctx.table;
    let columns_str = &ctx.columns_str;
    let placeholders_str = &ctx.placeholders_str;
    let id_name = ctx.id_name;
    let id_type = ctx.id_type;
    let soft_delete = ctx.soft_delete;
    let repo_name = format_ident!("{}TransactionRepo", entity_name);
    let marker = marker::generated();

    let bindings = super::sql::postgres::helpers::insert_bindings(entity.all_fields());
    let deleted_filter = if soft_delete {
        " AND deleted_at IS NULL"
    } else {
        ""
    };

    let create_method = if entity.create_fields().is_empty() {
        TokenStream::new()
    } else {
        quote! {
            /// Create a new entity within the transaction.
            pub async fn create(
                &mut self,
                dto: #create_dto
            ) -> Result<#entity_name, sqlx::Error> {
                let entity = #entity_name::from(dto);
                let insertable = #insertable_name::from(&entity);
                let row: #row_name = sqlx::query_as(
                    concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                )
                    #(#bindings)*
                    .fetch_one(&mut **self.tx).await?;
                Ok(#entity_name::from(row))
            }
        }
    };

    let update_method = if entity.update_fields().is_empty() {
        TokenStream::new()
    } else {
        let update_fields = entity.update_fields();
        let field_names: Vec<String> = update_fields.iter().map(|f| f.name_str()).collect();
        let field_refs: Vec<&str> = field_names.iter().map(String::as_str).collect();
        let set_clause = ctx.dialect.set_clause(&field_refs);
        let where_placeholder = ctx.dialect.placeholder(update_fields.len() + 1);
        let update_bindings = super::sql::postgres::helpers::update_bindings(&update_fields);

        quote! {
            /// Update an entity within the transaction.
            pub async fn update(
                &mut self,
                id: #id_type,
                dto: #update_dto
            ) -> Result<#entity_name, sqlx::Error> {
                let row: #row_name = sqlx::query_as(
                    &format!("UPDATE {} SET {} WHERE {} = {} RETURNING *",
                        #table, #set_clause, stringify!(#id_name), #where_placeholder)
                )
                    #(#update_bindings)*
                    .bind(&id)
                    .fetch_one(&mut **self.tx).await?;
                Ok(#entity_name::from(row))
            }
        }
    };

    let delete_sql = if soft_delete {
        quote! {
            let result = sqlx::query(&format!(
                "UPDATE {} SET deleted_at = NOW() WHERE {} = $1 AND deleted_at IS NULL",
                #table, stringify!(#id_name)
            )).bind(&id).execute(&mut **self.tx).await?;
            Ok(result.rows_affected() > 0)
        }
    } else {
        quote! {
            let result = sqlx::query(&format!(
                "DELETE FROM {} WHERE {} = $1",
                #table, stringify!(#id_name)
            )).bind(&id).execute(&mut **self.tx).await?;
            Ok(result.rows_affected() > 0)
        }
    };

    quote! {
        #marker
        /// Transaction repository adapter for #entity_name.
        ///
        /// Provides repository operations that execute within an active transaction.
        /// Access via `ctx.{entities}()` within a transaction closure.
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
                let row: Option<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} WHERE {} = $1{}",
                        #columns_str, #table, stringify!(#id_name), #deleted_filter)
                ).bind(&id).fetch_optional(&mut **self.tx).await?;
                Ok(row.map(#entity_name::from))
            }

            #update_method

            /// Delete an entity within the transaction.
            pub async fn delete(
                &mut self,
                id: #id_type
            ) -> Result<bool, sqlx::Error> {
                #delete_sql
            }

            /// List entities within the transaction.
            pub async fn list(
                &mut self,
                limit: i64,
                offset: i64
            ) -> Result<Vec<#entity_name>, sqlx::Error> {
                let where_clause = if #soft_delete { "WHERE deleted_at IS NULL " } else { "" };
                let rows: Vec<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} {}ORDER BY {} DESC LIMIT $1 OFFSET $2",
                        #columns_str, #table, where_clause, stringify!(#id_name))
                ).bind(limit).bind(offset).fetch_all(&mut **self.tx).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}

/// Generate the builder extension trait.
///
/// Creates an extension trait that adds `with_{entities}()` method to
/// `Transaction`. This method is chainable and returns self.
fn generate_builder_extension(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_snake = entity.name_str().to_case(Case::Snake);
    // Pluralize: add 's' for simple pluralization
    let plural = pluralize(&entity_snake);
    let method_name = format_ident!("with_{}", plural);
    let trait_name = format_ident!("TransactionWith{}", entity_name);
    let marker = marker::generated();

    quote! {
        #marker
        /// Extension trait to add #entity_name to a transaction builder.
        ///
        /// This is a fluent API method - it returns self for chaining.
        /// The actual repository is accessed via `ctx.{entities}()` in the closure.
        #vis trait #trait_name<'p> {
            /// Add #entity_name repository to the transaction.
            ///
            /// Returns self for chaining with other `with_*` calls.
            fn #method_name(self) -> Self;
        }

        impl<'p> #trait_name<'p> for entity_core::transaction::Transaction<'p, sqlx::PgPool> {
            fn #method_name(self) -> Self {
                self
            }
        }
    }
}

/// Generate the context extension trait.
///
/// Creates an extension trait that adds accessor method to
/// `TransactionContext`.
fn generate_context_extension(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let plural = pluralize(&entity_snake);
    let accessor_name = format_ident!("{}", plural);
    let trait_name = format_ident!("{}ContextExt", entity_name);
    let repo_name = format_ident!("{}TransactionRepo", entity_name);
    let marker = marker::generated();

    quote! {
        #marker
        /// Extension trait providing #entity_name access in transaction context.
        #vis trait #trait_name {
            /// Get repository adapter for #entity_name operations.
            fn #accessor_name(&mut self) -> #repo_name<'_>;
        }

        impl #trait_name for entity_core::transaction::TransactionContext {
            fn #accessor_name(&mut self) -> #repo_name<'_> {
                #repo_name::new(self.transaction())
            }
        }
    }
}

/// Simple pluralization - adds 's' to the end.
///
/// Handles some common cases:
/// - Words ending in 's', 'x', 'z', 'ch', 'sh' -> add 'es'
/// - Words ending in consonant + 'y' -> replace 'y' with 'ies'
/// - Otherwise -> add 's'
fn pluralize(word: &str) -> String {
    if word.ends_with('s')
        || word.ends_with('x')
        || word.ends_with('z')
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        format!("{}es", word)
    } else if let Some(without_y) = word.strip_suffix('y') {
        // Check if the letter before 'y' is a consonant
        if let Some(c) = without_y.chars().last()
            && !"aeiou".contains(c)
        {
            return format!("{}ies", without_y);
        }
        format!("{}s", word)
    } else {
        format!("{}s", word)
    }
}
