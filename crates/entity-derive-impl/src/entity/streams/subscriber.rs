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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriber_struct_generated() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", streams)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        let output_str = output.to_string();
        assert!(output_str.contains("UserSubscriber"));
        assert!(output_str.contains("PgListener"));
    }

    #[test]
    fn subscriber_has_new_method() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", streams)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        let output_str = output.to_string();
        assert!(output_str.contains("async fn new"));
        assert!(output_str.contains("PgPool"));
    }

    #[test]
    fn subscriber_has_recv_method() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", streams)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        let output_str = output.to_string();
        assert!(output_str.contains("async fn recv"));
        assert!(output_str.contains("UserEvent"));
    }

    #[test]
    fn subscriber_has_try_recv_method() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", streams)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        let output_str = output.to_string();
        assert!(output_str.contains("async fn try_recv"));
        assert!(output_str.contains("Option"));
    }

    #[test]
    fn subscriber_respects_visibility() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", streams)]
            pub(crate) struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        let output_str = output.to_string();
        assert!(output_str.contains("pub (crate) struct UserSubscriber"));
    }
}
