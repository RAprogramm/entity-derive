// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Postgres NOTIFY helpers for streaming.
//!
//! When the `streams` feature is enabled, generated CRUD methods publish
//! events via `pg_notify`. These helpers build the `pg_notify` SQL
//! fragments. The executor against which the notify runs is controlled
//! by the caller via `executor_tokens()`:
//!
//! - For atomic CRUD (the `update_method` / `create_method` / `delete_method`
//!   generators wrap the whole sequence in a transaction), the executor is
//!   `&mut *tx`, so `NOTIFY` participates in the same transaction — Postgres
//!   only broadcasts on commit, eliminating the commit-then-crash event-loss
//!   window.
//! - For non-streams entities `notify_*` returns an empty `TokenStream`, so the
//!   surrounding method stays a single SQL statement.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;

impl Context<'_> {
    /// Executor token used by `notify_*` and `fetch_old_for_update`.
    ///
    /// Streams-enabled CRUD wraps its DML in a `let mut tx = self.begin()...`
    /// block, so notify must execute on the transaction handle (`&mut *tx`),
    /// not on the pool. This keeps the notify atomic with the DML:
    /// Postgres buffers `NOTIFY` per transaction and broadcasts only on
    /// commit; on rollback the notify is discarded.
    fn executor_tokens() -> TokenStream {
        quote! { &mut *tx }
    }

    /// Generate `pg_notify` call for Created event.
    pub fn notify_created(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);
        let executor = Self::executor_tokens();

        quote! {
            let __event = #event_name::created(entity.clone());
            let __payload = ::serde_json::to_string(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query("SELECT pg_notify($1, $2)")
                .bind(#entity_name::CHANNEL)
                .bind(&__payload)
                .execute(#executor)
                .await?;
        }
    }

    /// Generate `pg_notify` call for Updated event.
    pub fn notify_updated(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);
        let executor = Self::executor_tokens();

        quote! {
            let __event = #event_name::updated(__old_entity, entity.clone());
            let __payload = ::serde_json::to_string(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query("SELECT pg_notify($1, $2)")
                .bind(#entity_name::CHANNEL)
                .bind(&__payload)
                .execute(#executor)
                .await?;
        }
    }

    /// Generate `pg_notify` call for `HardDeleted` event.
    pub fn notify_hard_deleted(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);
        let executor = Self::executor_tokens();

        quote! {
            let __event = #event_name::hard_deleted(id.clone());
            let __payload = ::serde_json::to_string(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query("SELECT pg_notify($1, $2)")
                .bind(#entity_name::CHANNEL)
                .bind(&__payload)
                .execute(#executor)
                .await?;
        }
    }

    /// Generate `pg_notify` call for `SoftDeleted` event.
    pub fn notify_soft_deleted(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);
        let executor = Self::executor_tokens();

        quote! {
            let __event = #event_name::SoftDeleted { id: id.clone() };
            let __payload = ::serde_json::to_string(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query("SELECT pg_notify($1, $2)")
                .bind(#entity_name::CHANNEL)
                .bind(&__payload)
                .execute(#executor)
                .await?;
        }
    }

    /// Generate fetch for old entity before update (for Updated event).
    ///
    /// Reads `SELECT ... FOR UPDATE` so the row is locked for the duration
    /// of the surrounding transaction. The old payload then matches the
    /// state immediately preceding the UPDATE — no concurrent writer can
    /// slip in between read and write.
    pub fn fetch_old_for_update(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let row_name = &self.row_name;
        let columns_str = &self.columns_str;
        let table = &self.table;
        let id_name = self.id_name;
        let placeholder = self.dialect.placeholder(1);
        let select_sql = format!(
            "SELECT {columns_str} FROM {table} WHERE {id_name} = {placeholder} FOR UPDATE"
        );

        quote! {
            let __old_row: #row_name = ::sqlx::query_as(#select_sql)
                .bind(&id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| ::sqlx::Error::RowNotFound)?;
            let __old_entity = #entity_name::from(__old_row);
        }
    }
}
