// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Router factory generation.
//!
//! Generates functions that create axum Routers for entity endpoints.
//!
//! # Generated Routers
//!
//! | Configuration | Generated Function | Type Parameter |
//! |---------------|-------------------|----------------|
//! | `handlers` | `{entity}_router<R>` | Repository trait |
//! | `commands` | `{entity}_commands_router<H>` | CommandHandler trait |
//!
//! # Example
//!
//! For `User` entity with both handlers and commands:
//!
//! ```rust,ignore
//! // CRUD router
//! pub fn user_router<R>() -> axum::Router<Arc<R>>
//! where
//!     R: UserRepository + 'static,
//! {
//!     axum::Router::new()
//!         .route("/users", post(create_user::<R>).get(list_user::<R>))
//!         .route("/users/:id", get(get_user::<R>).patch(update_user::<R>).delete(delete_user::<R>))
//! }
//!
//! // Commands router
//! pub fn user_commands_router<H>() -> axum::Router
//! where
//!     H: UserCommandHandler + 'static,
//!     H::Context: Default,
//! {
//!     axum::Router::new()
//!         .route("/users/register", post(register_user::<H>))
//! }
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::entity::parse::{CommandDef, CommandKindHint, EntityDef};

/// Generate all router factory functions.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let crud_router = generate_crud_router(entity);
    let commands_router = generate_commands_router(entity);

    quote! {
        #crud_router
        #commands_router
    }
}

/// Generate CRUD router for repository-based handlers.
fn generate_crud_router(entity: &EntityDef) -> TokenStream {
    if !entity.api_config().has_handlers() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let entity_snake = entity_name_str.to_case(Case::Snake);

    let router_fn = format_ident!("{}_router", entity_snake);
    let repo_trait = format_ident!("{}Repository", entity_name);

    let crud_routes = generate_crud_routes(entity);

    let doc = format!(
        "Create axum router for {} CRUD endpoints.\n\n\
         # Usage\n\n\
         ```rust,ignore\n\
         let pool = Arc::new(PgPool::connect(url).await?);\n\
         let app = Router::new()\n\
             .merge({}::<PgPool>())\n\
             .with_state(pool);\n\
         ```",
        entity_name, router_fn
    );

    quote! {
        #[doc = #doc]
        #vis fn #router_fn<R>() -> axum::Router<std::sync::Arc<R>>
        where
            R: #repo_trait + 'static,
        {
            axum::Router::new()
                #crud_routes
        }
    }
}

/// Generate CRUD route definitions based on enabled handlers.
fn generate_crud_routes(entity: &EntityDef) -> TokenStream {
    let handlers = entity.api_config().handlers();
    let snake = entity.name_str().to_case(Case::Snake);
    let collection_path = build_crud_collection_path(entity);
    let item_path = build_crud_item_path(entity);

    let create_handler = format_ident!("create_{}", snake);
    let get_handler = format_ident!("get_{}", snake);
    let update_handler = format_ident!("update_{}", snake);
    let delete_handler = format_ident!("delete_{}", snake);
    let list_handler = format_ident!("list_{}", snake);

    let mut collection_methods = Vec::new();
    if handlers.create {
        collection_methods.push(quote! { post(#create_handler::<R>) });
    }
    if handlers.list {
        collection_methods.push(quote! { get(#list_handler::<R>) });
    }

    let mut item_methods = Vec::new();
    if handlers.get {
        item_methods.push(quote! { get(#get_handler::<R>) });
    }
    if handlers.update {
        item_methods.push(quote! { patch(#update_handler::<R>) });
    }
    if handlers.delete {
        item_methods.push(quote! { delete(#delete_handler::<R>) });
    }

    let collection_route = if !collection_methods.is_empty() {
        let first = &collection_methods[0];
        let rest: Vec<_> = collection_methods.iter().skip(1).collect();
        quote! {
            .route(#collection_path, axum::routing::#first #(.#rest)*)
        }
    } else {
        TokenStream::new()
    };

    let item_route = if !item_methods.is_empty() {
        let first = &item_methods[0];
        let rest: Vec<_> = item_methods.iter().skip(1).collect();
        quote! {
            .route(#item_path, axum::routing::#first #(.#rest)*)
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #collection_route
        #item_route
    }
}

/// Build CRUD collection path (e.g., `/api/v1/users`).
fn build_crud_collection_path(entity: &EntityDef) -> String {
    let api_config = entity.api_config();
    let prefix = api_config.full_path_prefix();
    let entity_path = entity.name_str().to_case(Case::Kebab);

    let path = format!("{}/{}s", prefix, entity_path);
    path.replace("//", "/")
}

/// Build CRUD item path (e.g., `/api/v1/users/{id}`).
fn build_crud_item_path(entity: &EntityDef) -> String {
    let collection = build_crud_collection_path(entity);
    format!("{}/{{id}}", collection)
}

/// Generate commands router for command handler.
fn generate_commands_router(entity: &EntityDef) -> TokenStream {
    let commands = entity.command_defs();
    if commands.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let entity_snake = entity_name_str.to_case(Case::Snake);

    let router_fn = format_ident!("{}_commands_router", entity_snake);
    let handler_trait = format_ident!("{}CommandHandler", entity_name);

    let routes = generate_command_routes(entity, commands);

    let doc = format!(
        "Create axum router for {} command endpoints.\n\n\
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

/// Generate command route definitions.
fn generate_command_routes(entity: &EntityDef, commands: &[CommandDef]) -> TokenStream {
    let routes: Vec<TokenStream> = commands
        .iter()
        .map(|cmd| generate_command_route(entity, cmd))
        .collect();

    quote! { #(#routes)* }
}

/// Generate a single command route definition.
fn generate_command_route(entity: &EntityDef, cmd: &CommandDef) -> TokenStream {
    let path = build_command_path(entity, cmd);
    let handler_name = command_handler_name(entity, cmd);
    let method = axum_method_for_command(cmd);

    quote! {
        .route(#path, axum::routing::#method(#handler_name::<H>))
    }
}

/// Build command path (e.g., `/users/{id}/activate`).
fn build_command_path(entity: &EntityDef, cmd: &CommandDef) -> String {
    let api_config = entity.api_config();
    let prefix = api_config.full_path_prefix();
    let entity_path = entity.name_str().to_case(Case::Kebab);
    let cmd_path = cmd.name.to_string().to_case(Case::Kebab);

    let path = if cmd.requires_id {
        format!("{}/{}s/{{id}}/{}", prefix, entity_path, cmd_path)
    } else {
        format!("{}/{}s/{}", prefix, entity_path, cmd_path)
    };

    path.replace("//", "/")
}

/// Get command handler function name.
fn command_handler_name(entity: &EntityDef, cmd: &CommandDef) -> syn::Ident {
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let cmd_snake = cmd.name.to_string().to_case(Case::Snake);
    format_ident!("{}_{}", cmd_snake, entity_snake)
}

/// Get axum routing method for a command.
fn axum_method_for_command(cmd: &CommandDef) -> syn::Ident {
    match cmd.kind {
        CommandKindHint::Create => format_ident!("post"),
        CommandKindHint::Update => format_ident!("put"),
        CommandKindHint::Delete => format_ident!("delete"),
        CommandKindHint::Custom => format_ident!("post")
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use syn::Ident;

    use super::*;
    use crate::entity::parse::{CommandKindHint, CommandSource};

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
    fn crud_collection_path() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let path = build_crud_collection_path(&entity);
        assert_eq!(path, "/users");
    }

    #[test]
    fn crud_item_path() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let path = build_crud_item_path(&entity);
        assert_eq!(path, "/users/{id}");
    }

    #[test]
    fn crud_path_with_prefix() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", path_prefix = "/api/v1", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let path = build_crud_collection_path(&entity);
        assert_eq!(path, "/api/v1/users");
    }

    #[test]
    fn command_path_without_id() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", commands, api(tag = "Users"))]
            #[command(Register)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let cmd = create_test_command("Register", false, CommandKindHint::Create);
        let path = build_command_path(&entity, &cmd);
        assert_eq!(path, "/users/register");
    }

    #[test]
    fn command_path_with_id() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", commands, api(tag = "Users"))]
            #[command(UpdateEmail: email)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let cmd = create_test_command("UpdateEmail", true, CommandKindHint::Update);
        let path = build_command_path(&entity, &cmd);
        assert_eq!(path, "/users/{id}/update-email");
    }

    #[test]
    fn command_path_with_prefix() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", commands, api(tag = "Users", path_prefix = "/api/v2"))]
            #[command(Register)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let cmd = create_test_command("Register", false, CommandKindHint::Create);
        let path = build_command_path(&entity, &cmd);
        assert_eq!(path, "/api/v2/users/register");
    }

    #[test]
    fn command_handler_name_simple() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", commands, api(tag = "Users"))]
            #[command(Register)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let cmd = create_test_command("Register", false, CommandKindHint::Create);
        let name = command_handler_name(&entity, &cmd);
        assert_eq!(name.to_string(), "register_user");
    }

    #[test]
    fn command_handler_name_multi_word() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", commands, api(tag = "Users"))]
            #[command(UpdateEmail: email)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let cmd = create_test_command("UpdateEmail", true, CommandKindHint::Update);
        let name = command_handler_name(&entity, &cmd);
        assert_eq!(name.to_string(), "update_email_user");
    }

    #[test]
    fn axum_method_create() {
        let cmd = create_test_command("Register", false, CommandKindHint::Create);
        assert_eq!(axum_method_for_command(&cmd).to_string(), "post");
    }

    #[test]
    fn axum_method_update() {
        let cmd = create_test_command("Update", true, CommandKindHint::Update);
        assert_eq!(axum_method_for_command(&cmd).to_string(), "put");
    }

    #[test]
    fn axum_method_delete() {
        let cmd = create_test_command("Delete", true, CommandKindHint::Delete);
        assert_eq!(axum_method_for_command(&cmd).to_string(), "delete");
    }

    #[test]
    fn axum_method_custom() {
        let cmd = create_test_command("Transfer", false, CommandKindHint::Custom);
        assert_eq!(axum_method_for_command(&cmd).to_string(), "post");
    }

    #[test]
    fn generate_no_handlers_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users"))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate_crud_router(&entity);
        assert!(output.is_empty());
    }

    #[test]
    fn generate_no_commands_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate_commands_router(&entity);
        assert!(output.is_empty());
    }

    #[test]
    fn generate_crud_router_produces_output() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate_crud_router(&entity);
        let output_str = output.to_string();
        assert!(output_str.contains("user_router"));
        assert!(output_str.contains("UserRepository"));
    }

    #[test]
    fn generate_commands_router_produces_output() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", commands, api(tag = "Users"))]
            #[command(Register)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate_commands_router(&entity);
        let output_str = output.to_string();
        assert!(output_str.contains("user_commands_router"));
        assert!(output_str.contains("UserCommandHandler"));
    }

    #[test]
    fn generate_crud_routes_with_specific_handlers() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers(create, get)))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let routes = generate_crud_routes(&entity);
        let routes_str = routes.to_string();
        assert!(routes_str.contains("create_user"));
        assert!(routes_str.contains("get_user"));
        assert!(!routes_str.contains("delete_user"));
    }
}
