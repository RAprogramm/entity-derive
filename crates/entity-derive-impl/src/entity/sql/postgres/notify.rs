// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Postgres NOTIFY helpers for streaming.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;

impl Context<'_> {
    /// Generate pg_notify call for Created event.
    pub fn notify_created(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);

        quote! {
            let __event = #event_name::created(entity.clone());
            let __payload = ::serde_json::to_string(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query("SELECT pg_notify($1, $2)")
                .bind(#entity_name::CHANNEL)
                .bind(&__payload)
                .execute(self)
                .await?;
        }
    }

    /// Generate pg_notify call for Updated event.
    pub fn notify_updated(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);

        quote! {
            let __event = #event_name::updated(__old_entity, entity.clone());
            let __payload = ::serde_json::to_string(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query("SELECT pg_notify($1, $2)")
                .bind(#entity_name::CHANNEL)
                .bind(&__payload)
                .execute(self)
                .await?;
        }
    }

    /// Generate pg_notify call for HardDeleted event.
    pub fn notify_hard_deleted(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);

        quote! {
            let __event = #event_name::hard_deleted(id.clone());
            let __payload = ::serde_json::to_string(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query("SELECT pg_notify($1, $2)")
                .bind(#entity_name::CHANNEL)
                .bind(&__payload)
                .execute(self)
                .await?;
        }
    }

    /// Generate pg_notify call for SoftDeleted event.
    pub fn notify_soft_deleted(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);

        quote! {
            let __event = #event_name::soft_deleted(id.clone());
            let __payload = ::serde_json::to_string(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query("SELECT pg_notify($1, $2)")
                .bind(#entity_name::CHANNEL)
                .bind(&__payload)
                .execute(self)
                .await?;
        }
    }

    /// Generate fetch for old entity before update (for Updated event).
    pub fn fetch_old_for_update(&self) -> TokenStream {
        if !self.streams {
            return TokenStream::new();
        }

        let trait_name = &self.trait_name;

        quote! {
            let __old_entity = <Self as #trait_name>::find_by_id(self, id.clone())
                .await?
                .ok_or_else(|| ::sqlx::Error::RowNotFound)?;
        }
    }
}
