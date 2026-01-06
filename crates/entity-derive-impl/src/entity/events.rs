// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Lifecycle event enum generation.
//!
//! Generates an event enum for entities with `#[entity(events)]`.
//!
//! # Generated Code
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! #[derive(Debug, Clone)]
//! pub enum UserEvent {
//!     Created(User),
//!     Updated { old: User, new: User },
//!     SoftDeleted { id: Uuid },
//!     HardDeleted { id: Uuid },
//!     Restored { id: Uuid },
//! }
//!
//! impl entity_core::EntityEvent for UserEvent {
//!     type Id = Uuid;
//!
//!     fn kind(&self) -> entity_core::EventKind { ... }
//!     fn entity_id(&self) -> &Self::Id { ... }
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::EntityDef;
use crate::utils::marker;

/// Generates the lifecycle event enum for an entity.
///
/// Returns empty `TokenStream` if `events` is not enabled.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_events() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let event_name = format_ident!("{}Event", entity_name);

    let id_field = entity.id_field();
    let id_type = id_field.ty();
    let id_name = id_field.name();

    let soft_delete_variants = if entity.is_soft_delete() {
        quote! {
            /// Entity was soft-deleted (marked as deleted but not removed).
            SoftDeleted {
                /// ID of the soft-deleted entity.
                id: #id_type,
            },

            /// Entity was restored from soft-delete.
            Restored {
                /// ID of the restored entity.
                id: #id_type,
            },
        }
    } else {
        TokenStream::new()
    };

    let soft_delete_kind_arms = if entity.is_soft_delete() {
        quote! {
            Self::SoftDeleted { .. } => entity_core::EventKind::SoftDeleted,
            Self::Restored { .. } => entity_core::EventKind::Restored,
        }
    } else {
        TokenStream::new()
    };

    let soft_delete_id_arms = if entity.is_soft_delete() {
        quote! {
            Self::SoftDeleted { id } => id,
            Self::Restored { id } => id,
        }
    } else {
        TokenStream::new()
    };

    let marker = marker::generated();

    // Add serde derives when streams is enabled
    let serde_derives = if entity.has_streams() {
        quote! { , ::serde::Serialize, ::serde::Deserialize }
    } else {
        TokenStream::new()
    };

    quote! {
        #marker
        /// Lifecycle events for [`#entity_name`].
        ///
        /// Emitted during CRUD operations when `events` is enabled.
        /// Use these events for audit logging, cache invalidation,
        /// notifications, or the outbox pattern.
        #[derive(Debug, Clone #serde_derives)]
        #vis enum #event_name {
            /// Entity was created.
            Created(#entity_name),

            /// Entity was updated.
            Updated {
                /// Entity state before the update.
                old: #entity_name,
                /// Entity state after the update.
                new: #entity_name,
            },

            /// Entity was permanently deleted.
            HardDeleted {
                /// ID of the deleted entity.
                id: #id_type,
            },

            #soft_delete_variants
        }

        impl #event_name {
            /// Create a new Created event.
            pub fn created(entity: #entity_name) -> Self {
                Self::Created(entity)
            }

            /// Create a new Updated event.
            pub fn updated(old: #entity_name, new: #entity_name) -> Self {
                Self::Updated { old, new }
            }

            /// Create a new HardDeleted event.
            pub fn hard_deleted(id: #id_type) -> Self {
                Self::HardDeleted { id }
            }
        }

        impl entity_core::EntityEvent for #event_name {
            type Id = #id_type;

            fn kind(&self) -> entity_core::EventKind {
                match self {
                    Self::Created(_) => entity_core::EventKind::Created,
                    Self::Updated { .. } => entity_core::EventKind::Updated,
                    Self::HardDeleted { .. } => entity_core::EventKind::HardDeleted,
                    #soft_delete_kind_arms
                }
            }

            fn entity_id(&self) -> &Self::Id {
                match self {
                    Self::Created(e) => &e.#id_name,
                    Self::Updated { new, .. } => &new.#id_name,
                    Self::HardDeleted { id } => id,
                    #soft_delete_id_arms
                }
            }
        }
    }
}
