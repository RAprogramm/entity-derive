// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Router factory generation.
//!
//! Generates a function that creates an axum Router with all entity endpoints.
//!
//! # Generated Code
//!
//! For `User` entity with Register and UpdateEmail commands:
//!
//! ```rust,ignore
//! /// Create router for User entity endpoints.
//! pub fn user_router<H>() -> axum::Router
//! where
//!     H: UserCommandHandler + 'static,
//!     H::Context: Default,
//! {
//!     axum::Router::new()
//!         .route("/api/v1/users/register", axum::routing::post(register_user::<H>))
//!         .route("/api/v1/users/:id/update-email", axum::routing::put(update_email_user::<H>))
//! }
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::entity::parse::{CommandDef, CommandKindHint, EntityDef};

/// Generate the router factory function.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let commands = entity.command_defs();
    if commands.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let entity_snake = entity_name_str.to_case(Case::Snake);

    // Router function name: user_router
    let router_fn = format_ident!("{}_router", entity_snake);

    // Handler trait name: UserCommandHandler
    let handler_trait = format_ident!("{}CommandHandler", entity_name);

    // Generate route definitions
    let routes = generate_routes(entity, commands);

    let doc = format!(
        "Create axum router for {} entity endpoints.\n\n\
         # Usage\n\n\
         ```rust,ignore\n\
         let handler = Arc::new(MyHandler::new());\n\
         let app = Router::new()\n\
             .merge({}::<MyHandler>())\n\
             .layer(Extension(handler));\n\
         ```",
        entity_name, router_fn
    );

    quote! {
        #[doc = #doc]
        #vis fn #router_fn<H>() -> axum::Router
        where
            H: #handler_trait + 'static,
            H::Context: Default,
        {
            axum::Router::new()
                #routes
        }
    }
}

/// Generate all route definitions.
fn generate_routes(entity: &EntityDef, commands: &[CommandDef]) -> TokenStream {
    let routes: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| generate_route(entity, cmd))
        .collect();

    quote! { #(#routes)* }
}

/// Generate a single route definition.
fn generate_route(entity: &EntityDef, cmd: &CommandDef) -> TokenStream {
    let path = build_axum_path(entity, cmd);
    let handler_name = handler_function_name(entity, cmd);
    let method = axum_method_for_command(cmd);

    quote! {
        .route(#path, axum::routing::#method(#handler_name::<H>))
    }
}

/// Build the axum-style path (uses :id instead of {id}).
fn build_axum_path(entity: &EntityDef, cmd: &CommandDef) -> String {
    let api_config = entity.api_config();
    let prefix = api_config.full_path_prefix();
    let entity_path = entity.name_str().to_case(Case::Kebab);
    let cmd_path = cmd.name.to_string().to_case(Case::Kebab);

    let path = if cmd.requires_id {
        format!("{}/{}s/:id/{}", prefix, entity_path, cmd_path)
    } else {
        format!("{}/{}s/{}", prefix, entity_path, cmd_path)
    };

    // Normalize double slashes that can appear when prefix is empty
    path.replace("//", "/")
}

/// Get the handler function name.
fn handler_function_name(entity: &EntityDef, cmd: &CommandDef) -> syn::Ident {
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let cmd_snake = cmd.name.to_string().to_case(Case::Snake);
    format_ident!("{}_{}", cmd_snake, entity_snake)
}

/// Get the axum routing method for a command.
fn axum_method_for_command(cmd: &CommandDef) -> syn::Ident {
    match cmd.kind {
        CommandKindHint::Create => format_ident!("post"),
        CommandKindHint::Update => format_ident!("put"),
        CommandKindHint::Delete => format_ident!("delete"),
        CommandKindHint::Custom => format_ident!("post")
    }
}
