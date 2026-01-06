// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command enum generation.
//!
//! Generates the main command enum that wraps all command variants.
//!
//! # Generated Code
//!
//! For `User` entity with Register, UpdateEmail, Deactivate commands:
//!
//! ```rust,ignore
//! #[derive(Debug, Clone)]
//! pub enum UserCommand {
//!     Register(RegisterUser),
//!     UpdateEmail(UpdateEmailUser),
//!     Deactivate(DeactivateUser),
//! }
//!
//! impl entity_core::EntityCommand for UserCommand {
//!     fn kind(&self) -> entity_core::CommandKind { ... }
//!     fn name(&self) -> &'static str { ... }
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::struct_gen::{command_struct_name, custom_payload_type, uses_custom_payload};
use crate::{
    entity::parse::{CommandDef, CommandKindHint, EntityDef},
    utils::marker
};

/// Generate the command enum and its implementations.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let commands = entity.command_defs();
    if commands.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let enum_name = format_ident!("{}Command", entity_name);
    let marker = marker::generated();

    let variants = generate_variants(entity, commands);
    let kind_arms = generate_kind_arms(commands);
    let name_arms = generate_name_arms(commands);

    let doc = format!(
        "Command enum for [`{}`] entity.\n\n\
         Wraps all business commands for type-safe dispatch.",
        entity_name
    );

    quote! {
        #marker
        #[doc = #doc]
        #[derive(Debug, Clone)]
        #vis enum #enum_name {
            #variants
        }

        impl entity_core::EntityCommand for #enum_name {
            fn kind(&self) -> entity_core::CommandKind {
                match self {
                    #kind_arms
                }
            }

            fn name(&self) -> &'static str {
                match self {
                    #name_arms
                }
            }
        }
    }
}

/// Generate enum variants.
fn generate_variants(entity: &EntityDef, commands: &[CommandDef]) -> TokenStream {
    let variants: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| {
            let variant_name = &cmd.name;
            let payload_type = if uses_custom_payload(cmd) {
                let ty = custom_payload_type(cmd).unwrap();
                quote! { #ty }
            } else {
                let struct_name = command_struct_name(entity, cmd);
                quote! { #struct_name }
            };

            let doc = format!("{} command variant.", variant_name);

            quote! {
                #[doc = #doc]
                #variant_name(#payload_type),
            }
        })
        .collect();

    quote! { #(#variants)* }
}

/// Generate match arms for kind() method.
fn generate_kind_arms(commands: &[CommandDef]) -> TokenStream {
    let arms: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| {
            let variant_name = &cmd.name;
            let kind = match cmd.kind {
                CommandKindHint::Create => quote! { entity_core::CommandKind::Create },
                CommandKindHint::Update => quote! { entity_core::CommandKind::Update },
                CommandKindHint::Delete => quote! { entity_core::CommandKind::Delete },
                CommandKindHint::Custom => quote! { entity_core::CommandKind::Custom }
            };

            quote! {
                Self::#variant_name(_) => #kind,
            }
        })
        .collect();

    quote! { #(#arms)* }
}

/// Generate match arms for name() method.
fn generate_name_arms(commands: &[CommandDef]) -> TokenStream {
    let arms: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| {
            let variant_name = &cmd.name;
            let name_str = variant_name.to_string();

            quote! {
                Self::#variant_name(_) => #name_str,
            }
        })
        .collect();

    quote! { #(#arms)* }
}
