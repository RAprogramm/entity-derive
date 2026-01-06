// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Subscriber struct generation.
//!
//! Generates `{Entity}Subscriber` for async event streaming.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{entity::parse::EntityDef, utils::marker};

/// Generate the subscriber struct and implementation.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let subscriber_name = format_ident!("{}Subscriber", entity_name);
    let event_name = format_ident!("{}Event", entity_name);
    let marker = marker::generated();

    let doc = format!(
        "Subscriber for real-time [`{entity_name}`] change events.\n\n\
         Uses Postgres LISTEN/NOTIFY for cross-process notifications."
    );

    quote! {
        #marker
        #[doc = #doc]
        #vis struct #subscriber_name {
            listener: ::sqlx::postgres::PgListener,
        }

        impl #subscriber_name {
            /// Create a new subscriber connected to the database pool.
            ///
            /// Automatically subscribes to the entity's notification channel.
            pub async fn new(pool: &::sqlx::PgPool) -> Result<Self, ::sqlx::Error> {
                let mut listener = ::sqlx::postgres::PgListener::connect_with(pool).await?;
                listener.listen(#entity_name::CHANNEL).await?;
                Ok(Self { listener })
            }

            /// Receive the next event.
            ///
            /// Blocks until an event is available.
            pub async fn recv(
                &mut self,
            ) -> Result<#event_name, ::entity_core::stream::StreamError<::sqlx::Error>> {
                let notification = self
                    .listener
                    .recv()
                    .await
                    .map_err(::entity_core::stream::StreamError::Database)?;

                ::serde_json::from_str(notification.payload())
                    .map_err(|e| ::entity_core::stream::StreamError::Deserialize(e.to_string()))
            }

            /// Try to receive an event without blocking.
            ///
            /// Returns `None` if no event is immediately available.
            pub async fn try_recv(
                &mut self,
            ) -> Result<Option<#event_name>, ::entity_core::stream::StreamError<::sqlx::Error>> {
                match self.listener.try_recv().await {
                    Ok(Some(notification)) => {
                        let event = ::serde_json::from_str(notification.payload())
                            .map_err(|e| {
                                ::entity_core::stream::StreamError::Deserialize(e.to_string())
                            })?;
                        Ok(Some(event))
                    }
                    Ok(None) => Ok(None),
                    Err(e) => Err(::entity_core::stream::StreamError::Database(e)),
                }
            }
        }
    }
}
