// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Transactional-outbox enqueue fragments for `PostgreSQL`.
//!
//! When `#[entity(events(outbox))]` is set, every generated write
//! inserts the serialized `{Entity}Event` into the `entity_outbox`
//! table **in the same transaction** as the DML. A separate drainer
//! (`::entity_derive::outbox::OutboxDrainer`) delivers the rows with
//! retry/backoff, so events survive crashes that LISTEN/NOTIFY alone
//! would lose.
//!
//! # Generated Fragment Shape
//!
//! ```rust,ignore
//! let __event = UserEvent::created(entity.clone());
//! let __payload = ::serde_json::to_value(&__event)
//!     .expect("event serialization should not fail");
//! ::sqlx::query(
//!     "INSERT INTO entity_outbox (entity, kind, entity_id, payload) \
//!      VALUES ($1, $2, $3, $4)"
//! )
//!     .bind("users")
//!     .bind("created")
//!     .bind(entity.id.to_string())
//!     .bind(&__payload)
//!     .execute(&mut *tx)
//!     .await?;
//! ```
//!
//! The executor is always the surrounding transaction handle: outbox
//! implies transaction wrapping in the CRUD generators.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;

/// SQL inserting one event row into the shared outbox table.
pub const OUTBOX_INSERT_SQL: &str = "INSERT INTO entity_outbox (entity, kind, entity_id, payload) \
                                     VALUES ($1, $2, $3, $4)";

impl Context<'_> {
    /// Common enqueue fragment for an already-built `__event` binding.
    ///
    /// `kind` is the event kind column value; `entity_id_expr` yields the
    /// id of the affected row as a string.
    fn outbox_enqueue(&self, kind: &str, entity_id_expr: &TokenStream) -> TokenStream {
        if !self.outbox {
            return TokenStream::new();
        }

        let table = &self.table;

        quote! {
            let __outbox_payload = ::serde_json::to_value(&__event)
                .expect("event serialization should not fail");
            ::sqlx::query(#OUTBOX_INSERT_SQL)
                .bind(#table)
                .bind(#kind)
                .bind(#entity_id_expr)
                .bind(&__outbox_payload)
                .execute(&mut *tx)
                .await?;
        }
    }

    /// Enqueue a `Created` event for the freshly persisted `entity`.
    pub fn outbox_created(&self) -> TokenStream {
        if !self.outbox {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);
        let id_name = self.id_name;
        let enqueue = self.outbox_enqueue("created", &quote! { entity.#id_name.to_string() });

        quote! {
            {
                let __event = #event_name::created(entity.clone());
                #enqueue
            }
        }
    }

    /// Enqueue an `Updated` event from `old` and the persisted `entity`.
    pub fn outbox_updated(&self) -> TokenStream {
        if !self.outbox {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);
        let id_name = self.id_name;
        let enqueue = self.outbox_enqueue("updated", &quote! { entity.#id_name.to_string() });

        quote! {
            {
                let __event = #event_name::updated(__old_entity.clone(), entity.clone());
                #enqueue
            }
        }
    }

    /// Enqueue a delete event for the removed row id.
    ///
    /// Emits `SoftDeleted` for soft-delete entities and `HardDeleted`
    /// otherwise, matching the event enum variants.
    pub fn outbox_deleted(&self) -> TokenStream {
        if !self.outbox {
            return TokenStream::new();
        }

        let entity_name = self.entity_name;
        let event_name = format_ident!("{}Event", entity_name);
        let (constructor, kind) = if self.soft_delete {
            (format_ident!("soft_deleted"), "soft_deleted")
        } else {
            (format_ident!("hard_deleted"), "hard_deleted")
        };
        let enqueue = self.outbox_enqueue(kind, &quote! { id.to_string() });

        quote! {
            {
                let __event = #event_name::#constructor(id);
                #enqueue
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

    fn outbox_entity() -> EntityDef {
        parse_entity(quote! {
            #[entity(table = "users", events(outbox))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        })
    }

    #[test]
    fn created_fragment_inserts_into_outbox() {
        let entity = outbox_entity();
        let code = Context::new(&entity).outbox_created().to_string();
        assert!(code.contains("entity_outbox"));
        assert!(code.contains("\"created\""));
        assert!(code.contains("\"users\""));
    }

    #[test]
    fn deleted_fragment_uses_hard_deleted_without_soft_delete() {
        let entity = outbox_entity();
        let code = Context::new(&entity).outbox_deleted().to_string();
        assert!(code.contains("hard_deleted"));
    }

    #[test]
    fn deleted_fragment_uses_soft_deleted_with_soft_delete() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", events(outbox), soft_delete)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
                pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
            }
        });
        let code = Context::new(&entity).outbox_deleted().to_string();
        assert!(code.contains("soft_deleted"));
    }

    #[test]
    fn fragments_empty_without_outbox() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", events)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        });
        let ctx = Context::new(&entity);
        assert!(ctx.outbox_created().is_empty());
        assert!(ctx.outbox_updated().is_empty());
        assert!(ctx.outbox_deleted().is_empty());
    }
}
