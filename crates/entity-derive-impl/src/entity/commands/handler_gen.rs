// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command handler trait generation.
//!
//! Generates the handler trait with methods for each command.
//!
//! # Generated Code
//!
//! For `User` entity with Register, UpdateEmail, Deactivate commands:
//!
//! ```rust,ignore
//! #[async_trait]
//! pub trait UserCommandHandler: Send + Sync {
//!     type Error: std::error::Error + Send + Sync;
//!     type Context: Send + Sync;
//!
//!     async fn handle(&self, cmd: UserCommand, ctx: &Self::Context)
//!         -> Result<UserCommandResult, Self::Error>;
//!
//!     async fn handle_register(&self, cmd: RegisterUser, ctx: &Self::Context)
//!         -> Result<User, Self::Error>;
//!
//!     async fn handle_update_email(&self, cmd: UpdateEmailUser, ctx: &Self::Context)
//!         -> Result<User, Self::Error>;
//!
//!     async fn handle_deactivate(&self, cmd: DeactivateUser, ctx: &Self::Context)
//!         -> Result<(), Self::Error>;
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{
    result_gen::command_result_type,
    struct_gen::{command_struct_name, custom_payload_type, uses_custom_payload}
};
use crate::{
    entity::parse::{CommandDef, EntityDef},
    utils::marker
};

/// Generate the command handler trait.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let commands = entity.command_defs();
    if commands.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let trait_name = format_ident!("{}CommandHandler", entity_name);
    let command_enum = format_ident!("{}Command", entity_name);
    let result_enum = format_ident!("{}CommandResult", entity_name);
    let marker = marker::generated();

    let handler_methods = generate_handler_methods(entity, commands);
    let dispatch_arms = generate_dispatch_arms(entity, commands);

    let doc = format!(
        "Command handler trait for [`{}`] entity.\n\n\
         Implement this trait to handle business commands.\n\n\
         # Default Implementation\n\n\
         The `handle` method dispatches to individual handlers.\n\
         Override individual `handle_*` methods for your business logic.",
        entity_name
    );

    quote! {
        #marker
        #[doc = #doc]
        #[async_trait::async_trait]
        #vis trait #trait_name: Send + Sync {
            /// Error type for handler operations.
            type Error: std::error::Error + Send + Sync;

            /// Context type passed to handlers (e.g., database pool, user session).
            type Context: Send + Sync;

            /// Dispatch a command to its handler.
            ///
            /// Default implementation routes to individual `handle_*` methods.
            async fn handle(
                &self,
                cmd: #command_enum,
                ctx: &Self::Context
            ) -> Result<#result_enum, Self::Error> {
                match cmd {
                    #dispatch_arms
                }
            }

            #handler_methods
        }
    }
}

/// Generate individual handler methods.
fn generate_handler_methods(entity: &EntityDef, commands: &[CommandDef]) -> TokenStream {
    let methods: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| generate_handler_method(entity, cmd))
        .collect();

    quote! { #(#methods)* }
}

/// Generate a single handler method.
fn generate_handler_method(entity: &EntityDef, cmd: &CommandDef) -> TokenStream {
    let method_name = cmd.handler_method_name();

    let payload_type = if uses_custom_payload(cmd) {
        let ty = custom_payload_type(cmd).unwrap();
        quote! { #ty }
    } else {
        let struct_name = command_struct_name(entity, cmd);
        quote! { #struct_name }
    };

    let result_type = if let Some(ty) = command_result_type(entity, cmd) {
        quote! { #ty }
    } else {
        quote! { () }
    };

    let doc = format!(
        "Handle {} command.\n\n\
         Override this method to implement your business logic.",
        cmd.name
    );

    quote! {
        #[doc = #doc]
        async fn #method_name(
            &self,
            cmd: #payload_type,
            ctx: &Self::Context
        ) -> Result<#result_type, Self::Error>;
    }
}

/// Generate dispatch match arms for handle() method.
fn generate_dispatch_arms(entity: &EntityDef, commands: &[CommandDef]) -> TokenStream {
    let entity_name = entity.name();
    let command_enum = format_ident!("{}Command", entity_name);
    let result_enum = format_ident!("{}CommandResult", entity_name);

    let arms: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| {
            let variant_name = &cmd.name;
            let method_name = cmd.handler_method_name();

            let result_wrap = if command_result_type(entity, cmd).is_some() {
                quote! { #result_enum::#variant_name(result) }
            } else {
                quote! { { let _ = result; #result_enum::#variant_name } }
            };

            quote! {
                #command_enum::#variant_name(payload) => {
                    let result = self.#method_name(payload, ctx).await?;
                    Ok(#result_wrap)
                }
            }
        })
        .collect();

    quote! { #(#arms)* }
}
