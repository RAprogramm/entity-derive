// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command result enum generation.
//!
//! Generates the result enum for command execution results.
//!
//! # Generated Code
//!
//! For `User` entity with Register, UpdateEmail, Deactivate commands:
//!
//! ```rust,ignore
//! #[derive(Debug, Clone)]
//! pub enum UserCommandResult {
//!     Register(User),
//!     UpdateEmail(User),
//!     Deactivate,
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    entity::parse::{CommandDef, CommandKindHint, CommandSource, EntityDef},
    utils::marker
};

/// Generate the command result enum.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let commands = entity.command_defs();
    if commands.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let enum_name = format_ident!("{}CommandResult", entity_name);
    let marker = marker::generated();

    let variants = generate_variants(entity, commands);

    let doc = format!(
        "Result enum for [`{}`] command execution.\n\n\
         Each variant contains the result of the corresponding command.",
        entity_name
    );

    quote! {
        #marker
        #[doc = #doc]
        #[derive(Debug, Clone)]
        #vis enum #enum_name {
            #variants
        }
    }
}

/// Generate result enum variants.
fn generate_variants(entity: &EntityDef, commands: &[CommandDef]) -> TokenStream {
    let entity_name = entity.name();

    let variants: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| {
            let variant_name = &cmd.name;

            // Determine result type
            let result_type = if let Some(ref custom_type) = cmd.result_type {
                // Custom result type specified
                Some(quote! { #custom_type })
            } else {
                // Infer from command kind
                match cmd.kind {
                    CommandKindHint::Create | CommandKindHint::Update => {
                        Some(quote! { #entity_name })
                    }
                    CommandKindHint::Delete => None, // Unit result
                    CommandKindHint::Custom => {
                        // Custom commands with payload return entity by default
                        if matches!(cmd.source, CommandSource::Custom(_)) {
                            None // Let user specify via result attribute
                        } else {
                            Some(quote! { #entity_name })
                        }
                    }
                }
            };

            let doc = format!("Result of {} command.", variant_name);

            if let Some(ty) = result_type {
                quote! {
                    #[doc = #doc]
                    #variant_name(#ty),
                }
            } else {
                quote! {
                    #[doc = #doc]
                    #variant_name,
                }
            }
        })
        .collect();

    quote! { #(#variants)* }
}

/// Get result type for a command.
///
/// Returns `Some(Type)` for commands that return a value,
/// `None` for unit-result commands.
pub fn command_result_type(entity: &EntityDef, cmd: &CommandDef) -> Option<TokenStream> {
    let entity_name = entity.name();

    if let Some(ref custom_type) = cmd.result_type {
        Some(quote! { #custom_type })
    } else {
        match cmd.kind {
            CommandKindHint::Create | CommandKindHint::Update => Some(quote! { #entity_name }),
            CommandKindHint::Delete => None,
            CommandKindHint::Custom => {
                if matches!(cmd.source, CommandSource::Custom(_)) {
                    None
                } else {
                    Some(quote! { #entity_name })
                }
            }
        }
    }
}
