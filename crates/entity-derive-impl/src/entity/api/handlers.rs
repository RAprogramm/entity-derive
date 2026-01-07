// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Axum handler generation with utoipa annotations.
//!
//! Generates HTTP handlers for each command defined on the entity.
//! Each handler includes `#[utoipa::path]` annotations for OpenAPI
//! documentation.
//!
//! # Generated Handlers
//!
//! | Command Kind | HTTP Method | Path Pattern |
//! |--------------|-------------|--------------|
//! | Create (no id) | POST | `/{prefix}/{entity}` |
//! | Update (with id) | PUT | `/{prefix}/{entity}/{id}/{action}` |
//! | Delete (with id) | DELETE | `/{prefix}/{entity}/{id}` |
//! | Custom | POST | `/{prefix}/{entity}/{action}` |
//!
//! # Example
//!
//! For `#[command(Register)]` on `User`:
//!
//! ```rust,ignore
//! #[utoipa::path(
//!     post,
//!     path = "/api/v1/users/register",
//!     tag = "Users",
//!     request_body = RegisterUser,
//!     responses(
//!         (status = 200, body = User),
//!         (status = 400, description = "Validation error"),
//!         (status = 500, description = "Internal server error")
//!     )
//! )]
//! pub async fn register_user<H>(
//!     Extension(handler): Extension<Arc<H>>,
//!     Json(cmd): Json<RegisterUser>,
//! ) -> Result<Json<User>, ApiError>
//! where
//!     H: UserCommandHandler,
//! {
//!     let ctx = Default::default();
//!     let result = handler.handle_register(cmd, &ctx).await?;
//!     Ok(Json(result))
//! }
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::entity::parse::{CommandDef, CommandKindHint, EntityDef};

/// Generate all handler functions for the entity.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let commands = entity.command_defs();
    if commands.is_empty() {
        return TokenStream::new();
    }

    let handlers: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| generate_handler(entity, cmd))
        .collect();

    quote! { #(#handlers)* }
}

/// Generate a single handler function.
fn generate_handler(entity: &EntityDef, cmd: &CommandDef) -> TokenStream {
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();

    // Handler function name: register_user, update_email_user
    let handler_name = handler_function_name(entity, cmd);
    let handler_method = cmd.handler_method_name();

    // Command struct name: RegisterUser, UpdateEmailUser
    let command_struct = cmd.struct_name(&entity_name_str);

    // Handler trait name: UserCommandHandler
    let handler_trait = format_ident!("{}CommandHandler", entity_name);

    // Build the path for OpenAPI
    let path = build_path(entity, cmd);

    // HTTP method based on command kind
    let http_method = http_method_for_command(cmd);
    let http_method_ident = format_ident!("{}", http_method);

    // Tag for OpenAPI grouping
    let tag = api_config.tag_or_default(&entity_name_str);

    // Security configuration
    // Priority: command-level override > entity-level public list > entity-level
    // default
    let security_attr = if cmd.is_public() {
        // Command explicitly marked as public
        quote! {}
    } else if let Some(cmd_security) = cmd.security() {
        // Command has explicit security override
        let security_name = security_scheme_name(cmd_security);
        quote! { security(#security_name = []) }
    } else if api_config.is_public_command(&cmd.name.to_string()) {
        // Command is in entity-level public list
        quote! {}
    } else if let Some(security) = &api_config.security {
        // Use entity-level default security
        let security_name = security_scheme_name(security);
        quote! { security(#security_name = []) }
    } else {
        quote! {}
    };

    // Determine response type
    let (response_type, response_body) = response_type_for_command(entity, cmd);

    // Deprecated flag from api config
    let deprecated_attr = if api_config.is_deprecated() {
        quote! { , deprecated = true }
    } else {
        quote! {}
    };

    // Build utoipa path attribute
    let utoipa_attr = if security_attr.is_empty() {
        quote! {
            #[utoipa::path(
                #http_method_ident,
                path = #path,
                tag = #tag,
                request_body = #command_struct,
                responses(
                    (status = 200, body = #response_body, description = "Success"),
                    (status = 400, description = "Validation error"),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    } else {
        quote! {
            #[utoipa::path(
                #http_method_ident,
                path = #path,
                tag = #tag,
                request_body = #command_struct,
                responses(
                    (status = 200, body = #response_body, description = "Success"),
                    (status = 400, description = "Validation error"),
                    (status = 401, description = "Unauthorized"),
                    (status = 500, description = "Internal server error")
                ),
                #security_attr
                #deprecated_attr
            )]
        }
    };

    // Generate handler based on whether it requires ID
    if cmd.requires_id {
        generate_handler_with_id(
            entity,
            cmd,
            &handler_name,
            &handler_method,
            &command_struct,
            &handler_trait,
            &response_type,
            &utoipa_attr
        )
    } else {
        generate_handler_without_id(
            entity,
            cmd,
            &handler_name,
            &handler_method,
            &command_struct,
            &handler_trait,
            &response_type,
            &utoipa_attr
        )
    }
}

/// Generate handler for commands that don't require an ID (e.g., Register).
#[allow(clippy::too_many_arguments)]
fn generate_handler_without_id(
    entity: &EntityDef,
    cmd: &CommandDef,
    handler_name: &syn::Ident,
    handler_method: &syn::Ident,
    command_struct: &syn::Ident,
    handler_trait: &syn::Ident,
    response_type: &TokenStream,
    utoipa_attr: &TokenStream
) -> TokenStream {
    let vis = &entity.vis;
    let doc = format!(
        "HTTP handler for {} command.\n\n\
         Generated by entity-derive.",
        cmd.name
    );

    quote! {
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<H>(
            axum::extract::Extension(handler): axum::extract::Extension<std::sync::Arc<H>>,
            axum::extract::Json(cmd): axum::extract::Json<#command_struct>,
        ) -> Result<axum::response::Json<#response_type>, axum::http::StatusCode>
        where
            H: #handler_trait + 'static,
            H::Context: Default,
        {
            let ctx = H::Context::default();
            match handler.#handler_method(cmd, &ctx).await {
                Ok(result) => Ok(axum::response::Json(result)),
                Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// Generate handler for commands that require an ID (e.g., UpdateEmail).
#[allow(clippy::too_many_arguments)]
fn generate_handler_with_id(
    entity: &EntityDef,
    cmd: &CommandDef,
    handler_name: &syn::Ident,
    handler_method: &syn::Ident,
    command_struct: &syn::Ident,
    handler_trait: &syn::Ident,
    response_type: &TokenStream,
    utoipa_attr: &TokenStream
) -> TokenStream {
    let vis = &entity.vis;
    let id_field = entity.id_field();
    let id_type = &id_field.ty;
    let doc = format!(
        "HTTP handler for {} command.\n\n\
         Generated by entity-derive.",
        cmd.name
    );

    quote! {
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<H>(
            axum::extract::Extension(handler): axum::extract::Extension<std::sync::Arc<H>>,
            axum::extract::Path(id): axum::extract::Path<#id_type>,
            axum::extract::Json(mut cmd): axum::extract::Json<#command_struct>,
        ) -> Result<axum::response::Json<#response_type>, axum::http::StatusCode>
        where
            H: #handler_trait + 'static,
            H::Context: Default,
        {
            cmd.id = id;
            let ctx = H::Context::default();
            match handler.#handler_method(cmd, &ctx).await {
                Ok(result) => Ok(axum::response::Json(result)),
                Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// Get the handler function name.
///
/// Example: `register_user`, `update_email_user`
fn handler_function_name(entity: &EntityDef, cmd: &CommandDef) -> syn::Ident {
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let cmd_snake = cmd.name.to_string().to_case(Case::Snake);
    format_ident!("{}_{}", cmd_snake, entity_snake)
}

/// Build the URL path for a command.
fn build_path(entity: &EntityDef, cmd: &CommandDef) -> String {
    let api_config = entity.api_config();
    let prefix = api_config.full_path_prefix();
    let entity_path = entity.name_str().to_case(Case::Kebab);
    let cmd_path = cmd.name.to_string().to_case(Case::Kebab);

    if cmd.requires_id {
        format!("{}/{}/{{id}}/{}", prefix, entity_path, cmd_path)
    } else {
        format!("{}/{}/{}", prefix, entity_path, cmd_path)
    }
}

/// Get HTTP method for a command based on its kind.
fn http_method_for_command(cmd: &CommandDef) -> &'static str {
    match cmd.kind {
        CommandKindHint::Create => "post",
        CommandKindHint::Update => "put",
        CommandKindHint::Delete => "delete",
        CommandKindHint::Custom => "post"
    }
}

/// Map security scheme name to OpenAPI security scheme identifier.
fn security_scheme_name(scheme: &str) -> &'static str {
    match scheme {
        "bearer" => "bearer_auth",
        "api_key" => "api_key",
        "admin" => "admin_auth",
        "oauth2" => "oauth2",
        _ => "bearer_auth"
    }
}

/// Get the response type for a command.
fn response_type_for_command(entity: &EntityDef, cmd: &CommandDef) -> (TokenStream, TokenStream) {
    let entity_name = entity.name();

    if let Some(ref result_type) = cmd.result_type {
        (quote! { #result_type }, quote! { #result_type })
    } else {
        match cmd.kind {
            CommandKindHint::Delete => (quote! { () }, quote! { () }),
            _ => (quote! { #entity_name }, quote! { #entity_name })
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use syn::Ident;

    use super::*;
    use crate::entity::parse::{CommandDef, CommandSource};

    fn create_test_command(name: &str, requires_id: bool, kind: CommandKindHint) -> CommandDef {
        CommandDef {
            name: Ident::new(name, Span::call_site()),
            source: CommandSource::Create,
            requires_id,
            result_type: None,
            kind,
            security: None
        }
    }

    #[test]
    fn http_method_create() {
        let cmd = create_test_command("Register", false, CommandKindHint::Create);
        assert_eq!(http_method_for_command(&cmd), "post");
    }

    #[test]
    fn http_method_update() {
        let cmd = create_test_command("Update", true, CommandKindHint::Update);
        assert_eq!(http_method_for_command(&cmd), "put");
    }

    #[test]
    fn http_method_delete() {
        let cmd = create_test_command("Delete", true, CommandKindHint::Delete);
        assert_eq!(http_method_for_command(&cmd), "delete");
    }

    #[test]
    fn http_method_custom() {
        let cmd = create_test_command("Transfer", false, CommandKindHint::Custom);
        assert_eq!(http_method_for_command(&cmd), "post");
    }

    #[test]
    fn security_scheme_bearer() {
        assert_eq!(security_scheme_name("bearer"), "bearer_auth");
    }

    #[test]
    fn security_scheme_api_key() {
        assert_eq!(security_scheme_name("api_key"), "api_key");
    }

    #[test]
    fn security_scheme_admin() {
        assert_eq!(security_scheme_name("admin"), "admin_auth");
    }

    #[test]
    fn security_scheme_oauth2() {
        assert_eq!(security_scheme_name("oauth2"), "oauth2");
    }

    #[test]
    fn security_scheme_unknown_defaults_to_bearer() {
        assert_eq!(security_scheme_name("unknown"), "bearer_auth");
    }
}
