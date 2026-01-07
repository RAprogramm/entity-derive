// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Real-time streaming code generation.
//!
//! Generates streaming infrastructure for entities with `#[entity(streams)]`.
//!
//! # Generated Code
//!
//! | Type | Purpose |
//! |------|---------|
//! | `{Entity}::CHANNEL` | Postgres NOTIFY channel name |
//! | `{Entity}Subscriber` | Async event subscriber |

mod subscriber;

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::EntityDef;

/// Main entry point for streams code generation.
///
/// Returns empty `TokenStream` if `streams` is not enabled.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_streams() {
        return TokenStream::new();
    }

    let channel_const = generate_channel_const(entity);
    let subscriber = subscriber::generate(entity);

    quote! {
        #channel_const
        #subscriber
    }
}

/// Generate the CHANNEL constant for Postgres NOTIFY.
fn generate_channel_const(entity: &EntityDef) -> TokenStream {
    let entity_name = entity.name();
    let channel_name = format!("entity_{}", entity.table);

    quote! {
        impl #entity_name {
            /// Postgres NOTIFY channel name for this entity.
            pub const CHANNEL: &'static str = #channel_name;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_no_streams_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        assert!(output.is_empty());
    }

    #[test]
    fn generate_with_streams() {
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
        assert!(output_str.contains("CHANNEL"));
        assert!(output_str.contains("entity_users"));
        assert!(output_str.contains("UserSubscriber"));
    }

    #[test]
    fn channel_const_format() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "blog_posts", streams)]
            pub struct BlogPost {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate_channel_const(&entity);
        let output_str = output.to_string();
        assert!(output_str.contains("entity_blog_posts"));
    }
}
