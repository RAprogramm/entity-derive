// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI path operations generation.
//!
//! This module generates CRUD path operations for the OpenAPI specification.
//! Path operations define the available endpoints, their HTTP methods,
//! parameters, request/response bodies, and security requirements.
//!
//! # OpenAPI Paths Object
//!
//! The paths object is the core of the OpenAPI specification:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                      OpenAPI Paths                                  │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  /users:                       # Collection path                    │
//! │  ├─► POST   create_user        # Create new entity                  │
//! │  │   ├─► requestBody: CreateUserRequest                             │
//! │  │   ├─► responses: 201, 400, 401, 500                              │
//! │  │   └─► security: bearerAuth                                       │
//! │  │                                                                  │
//! │  └─► GET    list_user          # List entities with pagination      │
//! │      ├─► parameters: limit, offset                                  │
//! │      ├─► responses: 200, 401, 500                                   │
//! │      └─► security: bearerAuth                                       │
//! │                                                                     │
//! │  /users/{id}:                  # Item path                          │
//! │  ├─► GET    get_user           # Get single entity                  │
//! │  │   ├─► parameters: id (path)                                      │
//! │  │   ├─► responses: 200, 401, 404, 500                              │
//! │  │   └─► security: bearerAuth                                       │
//! │  │                                                                  │
//! │  ├─► PATCH  update_user        # Partial update                     │
//! │  │   ├─► parameters: id (path)                                      │
//! │  │   ├─► requestBody: UpdateUserRequest                             │
//! │  │   ├─► responses: 200, 400, 401, 404, 500                         │
//! │  │   └─► security: bearerAuth                                       │
//! │  │                                                                  │
//! │  └─► DELETE delete_user        # Remove entity                      │
//! │      ├─► parameters: id (path)                                      │
//! │      ├─► responses: 204, 401, 404, 500                              │
//! │      └─► security: bearerAuth                                       │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Path Patterns
//!
//! Two URL patterns are used following REST conventions:
//!
//! | Pattern | Name | Operations | Example |
//! |---------|------|------------|---------|
//! | `/{prefix}/{entities}` | Collection | POST, GET | `/api/v1/users` |
//! | `/{prefix}/{entities}/{id}` | Item | GET, PATCH, DELETE | `/api/v1/users/{id}` |
//!
//! # Path Configuration
//!
//! Paths are constructed from entity configuration:
//!
//! ```rust,ignore
//! #[entity(
//!     table = "users",
//!     api(
//!         prefix = "api",           // Base prefix
//!         api_version = "v1",       // Version segment
//!         handlers(get, list)       // Enabled operations
//!     )
//! )]
//! pub struct User { ... }
//!
//! // Generated paths:
//! // GET  /api/v1/users
//! // GET  /api/v1/users/{id}
//! ```
//!
//! # Operation Components
//!
//! Each operation includes:
//!
//! | Component | Description | Example |
//! |-----------|-------------|---------|
//! | `operationId` | Unique identifier | `create_user` |
//! | `summary` | Short description | "Create a new User" |
//! | `description` | Detailed description | "Creates a new User entity" |
//! | `tag` | API grouping | "Users" |
//! | `parameters` | Path/query params | `id: Uuid` |
//! | `requestBody` | Request schema | `CreateUserRequest` |
//! | `responses` | Response codes/bodies | 200, 404, 500 |
//! | `security` | Auth requirements | `bearerAuth` |
//!
//! # Response Codes
//!
//! Standard HTTP response codes per operation:
//!
//! | Operation | Success | Client Error | Server Error |
//! |-----------|---------|--------------|--------------|
//! | Create | 201 | 400, 401 | 500 |
//! | List | 200 | 401 | 500 |
//! | Get | 200 | 401, 404 | 500 |
//! | Update | 200 | 400, 401, 404 | 500 |
//! | Delete | 204 | 401, 404 | 500 |

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::security::security_scheme_name;
use crate::entity::parse::{CommandDef, EntityDef};

/// Generates code to add CRUD path operations to the OpenAPI specification.
///
/// This function produces code that registers all enabled CRUD operations
/// as paths in the OpenAPI spec. Each operation is fully documented with
/// parameters, request bodies, responses, and security requirements.
///
/// # Arguments
///
/// * `entity` - The parsed entity definition containing handler configuration
///
/// # Returns
///
/// A `TokenStream` containing code to add paths via
/// `openapi.paths.add_path_operation()`.
///
/// # Conditional Generation
///
/// Only enabled handlers generate path operations:
///
/// ```text
/// HandlerConfig
///     │
///     ├─► create == true ──► POST /entities
///     ├─► list == true ────► GET /entities
///     ├─► get == true ─────► GET /entities/{id}
///     ├─► update == true ──► PATCH /entities/{id}
///     └─► delete == true ──► DELETE /entities/{id}
/// ```
///
/// # Generated Code Structure
///
/// ```rust,ignore
/// // Common setup
/// let error_response = |desc: &str| -> response::Response { ... };
/// let security_req: Option<Vec<SecurityRequirement>> = ...;
/// let id_param = path::ParameterBuilder::new()...;
///
/// // Create operation (if enabled)
/// let create_op = path::OperationBuilder::new()
///     .operation_id(Some("create_user"))
///     .tag("Users")
///     .request_body(Some(...))
///     .response("201", ...)
///     .build();
/// openapi.paths.add_path_operation("/users", vec![HttpMethod::Post], create_op);
///
/// // Similar for other operations...
/// ```
///
/// # Security Handling
///
/// When security is configured:
/// - Each operation includes security requirements
/// - 401 response is added to all operations
/// - Lock icon appears in Swagger UI
pub fn generate_paths_code(entity: &EntityDef) -> TokenStream {
    let api_config = entity.api_config();
    let handlers = api_config.handlers();
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let id_field = entity.id_field();
    let id_type = &id_field.ty;

    let tag = api_config.tag_or_default(&entity_name_str);
    let collection_path = build_collection_path(entity);
    let item_path = build_item_path(entity);

    let response_schema = entity.ident_with("", "Response");
    let create_schema = entity.ident_with("Create", "Request");
    let update_schema = entity.ident_with("Update", "Request");

    let response_ref = response_schema.to_string();
    let create_ref = create_schema.to_string();
    let update_ref = update_schema.to_string();

    let security_req = if let Some(security) = &api_config.security {
        let scheme_name = security_scheme_name(security);
        quote! {
            Some(vec![security::SecurityRequirement::new::<_, _, &str>(#scheme_name, [])])
        }
    } else {
        quote! { None }
    };

    let needs_id_param = handlers.get || handlers.update || handlers.delete;
    let id_type_str = quote!(#id_type).to_string().replace(' ', "");
    let id_schema_type = if id_type_str.contains("Uuid") {
        quote! {
            ObjectBuilder::new()
                .schema_type(schema::Type::String)
                .format(Some(schema::SchemaFormat::Custom("uuid".into())))
                .build()
        }
    } else {
        quote! {
            ObjectBuilder::new()
                .schema_type(schema::Type::String)
                .build()
        }
    };

    let create_op_id = format!("create_{}", entity_name_str.to_case(Case::Snake));
    let get_op_id = format!("get_{}", entity_name_str.to_case(Case::Snake));
    let update_op_id = format!("update_{}", entity_name_str.to_case(Case::Snake));
    let delete_op_id = format!("delete_{}", entity_name_str.to_case(Case::Snake));
    let list_op_id = format!("list_{}", entity_name_str.to_case(Case::Snake));

    let create_summary = format!("Create a new {}", entity_name);
    let get_summary = format!("Get {} by ID", entity_name);
    let update_summary = format!("Update {} by ID", entity_name);
    let delete_summary = format!("Delete {} by ID", entity_name);
    let list_summary = format!("List all {}", entity_name);

    let create_desc = format!("Creates a new {} entity", entity_name);
    let get_desc = format!("Retrieves a {} by its unique identifier", entity_name);
    let update_desc = format!("Updates an existing {} by ID", entity_name);
    let delete_desc = format!("Deletes a {} by ID", entity_name);
    let list_desc = format!("Returns a paginated list of {} entities", entity_name);

    let id_param_desc = format!("{} unique identifier", entity_name);
    let created_desc = format!("{} created successfully", entity_name);
    let found_desc = format!("{} found", entity_name);
    let updated_desc = format!("{} updated successfully", entity_name);
    let deleted_desc = format!("{} deleted successfully", entity_name);
    let list_desc_resp = format!("List of {} entities", entity_name);
    let not_found_desc = format!("{} not found", entity_name);

    let common_code = quote! {
        let error_response = |desc: &str| -> response::Response {
            response::ResponseBuilder::new()
                .description(desc)
                .content("application/json",
                    content::ContentBuilder::new()
                        .schema(Some(Ref::from_schema_name("ErrorResponse")))
                        .build()
                )
                .build()
        };

        let security_req: Option<Vec<security::SecurityRequirement>> = #security_req;
    };

    let id_param_code = if needs_id_param {
        quote! {
            let id_param = path::ParameterBuilder::new()
                .name("id")
                .parameter_in(path::ParameterIn::Path)
                .required(utoipa::openapi::Required::True)
                .description(Some(#id_param_desc))
                .schema(Some(#id_schema_type))
                .build();
        }
    } else {
        TokenStream::new()
    };

    let create_code = if handlers.create {
        quote! {
            let create_op = {
                let mut op = path::OperationBuilder::new()
                    .operation_id(Some(#create_op_id))
                    .tag(#tag)
                    .summary(Some(#create_summary))
                    .description(Some(#create_desc))
                    .request_body(Some(
                        request_body::RequestBodyBuilder::new()
                            .description(Some("Request body"))
                            .required(Some(utoipa::openapi::Required::True))
                            .content("application/json",
                                content::ContentBuilder::new()
                                    .schema(Some(Ref::from_schema_name(#create_ref)))
                                    .build()
                            )
                            .build()
                    ))
                    .response("201",
                        response::ResponseBuilder::new()
                            .description(#created_desc)
                            .content("application/json",
                                content::ContentBuilder::new()
                                    .schema(Some(Ref::from_schema_name(#response_ref)))
                                    .build()
                            )
                            .build()
                    )
                    .response("400", error_response("Invalid request data"))
                    .response("500", error_response("Internal server error"));
                if let Some(ref sec) = security_req {
                    op = op.securities(Some(sec.clone()))
                        .response("401", error_response("Authentication required"));
                }
                op.build()
            };
            openapi.paths.add_path_operation(#collection_path, vec![path::HttpMethod::Post], create_op);
        }
    } else {
        TokenStream::new()
    };

    let list_code = if handlers.list {
        quote! {
            let limit_param = path::ParameterBuilder::new()
                .name("limit")
                .parameter_in(path::ParameterIn::Query)
                .required(utoipa::openapi::Required::False)
                .description(Some("Maximum number of items to return (default: 100)"))
                .schema(Some(ObjectBuilder::new().schema_type(schema::Type::Integer).build()))
                .build();

            let offset_param = path::ParameterBuilder::new()
                .name("offset")
                .parameter_in(path::ParameterIn::Query)
                .required(utoipa::openapi::Required::False)
                .description(Some("Number of items to skip for pagination"))
                .schema(Some(ObjectBuilder::new().schema_type(schema::Type::Integer).build()))
                .build();

            let list_op = {
                let mut op = path::OperationBuilder::new()
                    .operation_id(Some(#list_op_id))
                    .tag(#tag)
                    .summary(Some(#list_summary))
                    .description(Some(#list_desc))
                    .parameter(limit_param)
                    .parameter(offset_param)
                    .response("200",
                        response::ResponseBuilder::new()
                            .description(#list_desc_resp)
                            .content("application/json",
                                content::ContentBuilder::new()
                                    .schema(Some(
                                        schema::ArrayBuilder::new()
                                            .items(Ref::from_schema_name(#response_ref))
                                            .build()
                                    ))
                                    .build()
                            )
                            .build()
                    )
                    .response("500", error_response("Internal server error"));
                if let Some(ref sec) = security_req {
                    op = op.securities(Some(sec.clone()))
                        .response("401", error_response("Authentication required"));
                }
                op.build()
            };
            openapi.paths.add_path_operation(#collection_path, vec![path::HttpMethod::Get], list_op);
        }
    } else {
        TokenStream::new()
    };

    let get_code = if handlers.get {
        quote! {
            let get_op = {
                let mut op = path::OperationBuilder::new()
                    .operation_id(Some(#get_op_id))
                    .tag(#tag)
                    .summary(Some(#get_summary))
                    .description(Some(#get_desc))
                    .parameter(id_param.clone())
                    .response("200",
                        response::ResponseBuilder::new()
                            .description(#found_desc)
                            .content("application/json",
                                content::ContentBuilder::new()
                                    .schema(Some(Ref::from_schema_name(#response_ref)))
                                    .build()
                            )
                            .build()
                    )
                    .response("404", error_response(#not_found_desc))
                    .response("500", error_response("Internal server error"));
                if let Some(ref sec) = security_req {
                    op = op.securities(Some(sec.clone()))
                        .response("401", error_response("Authentication required"));
                }
                op.build()
            };
            openapi.paths.add_path_operation(#item_path, vec![path::HttpMethod::Get], get_op);
        }
    } else {
        TokenStream::new()
    };

    let update_code = if handlers.update {
        quote! {
            let update_op = {
                let mut op = path::OperationBuilder::new()
                    .operation_id(Some(#update_op_id))
                    .tag(#tag)
                    .summary(Some(#update_summary))
                    .description(Some(#update_desc))
                    .parameter(id_param.clone())
                    .request_body(Some(
                        request_body::RequestBodyBuilder::new()
                            .description(Some("Fields to update"))
                            .required(Some(utoipa::openapi::Required::True))
                            .content("application/json",
                                content::ContentBuilder::new()
                                    .schema(Some(Ref::from_schema_name(#update_ref)))
                                    .build()
                            )
                            .build()
                    ))
                    .response("200",
                        response::ResponseBuilder::new()
                            .description(#updated_desc)
                            .content("application/json",
                                content::ContentBuilder::new()
                                    .schema(Some(Ref::from_schema_name(#response_ref)))
                                    .build()
                            )
                            .build()
                    )
                    .response("400", error_response("Invalid request data"))
                    .response("404", error_response(#not_found_desc))
                    .response("500", error_response("Internal server error"));
                if let Some(ref sec) = security_req {
                    op = op.securities(Some(sec.clone()))
                        .response("401", error_response("Authentication required"));
                }
                op.build()
            };
            openapi.paths.add_path_operation(#item_path, vec![path::HttpMethod::Patch], update_op);
        }
    } else {
        TokenStream::new()
    };

    let delete_code = if handlers.delete {
        quote! {
            let delete_op = {
                let mut op = path::OperationBuilder::new()
                    .operation_id(Some(#delete_op_id))
                    .tag(#tag)
                    .summary(Some(#delete_summary))
                    .description(Some(#delete_desc))
                    .parameter(id_param.clone())
                    .response("204",
                        response::ResponseBuilder::new()
                            .description(#deleted_desc)
                            .build()
                    )
                    .response("404", error_response(#not_found_desc))
                    .response("500", error_response("Internal server error"));
                if let Some(ref sec) = security_req {
                    op = op.securities(Some(sec.clone()))
                        .response("401", error_response("Authentication required"));
                }
                op.build()
            };
            openapi.paths.add_path_operation(#item_path, vec![path::HttpMethod::Delete], delete_op);
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #common_code
        #id_param_code
        #create_code
        #list_code
        #get_code
        #update_code
        #delete_code
    }
}

/// Builds the collection path for an entity (e.g., `/users`).
///
/// Collection paths are used for operations that affect multiple entities
/// or create new entities: `POST` (create) and `GET` (list).
///
/// # Arguments
///
/// * `entity` - The parsed entity definition
///
/// # Returns
///
/// A path string with the format `/{prefix}/{version}/{entity}s`.
///
/// # Path Construction
///
/// ```text
/// ApiConfig               Result
///     │
///     ├─► prefix: "api"
///     │       │
///     ├─► api_version: "v1"     ─────►  /api/v1/users
///     │       │
///     └─► entity: "User"
///            └─► kebab-case + plural
/// ```
///
/// # Examples
///
/// | Entity | Prefix | Version | Result |
/// |--------|--------|---------|--------|
/// | `User` | - | - | `/users` |
/// | `User` | `api` | - | `/api/users` |
/// | `User` | `api` | `v1` | `/api/v1/users` |
/// | `BlogPost` | - | - | `/blog-posts` |
/// | `OrderItem` | `api` | `v2` | `/api/v2/order-items` |
///
/// # Pluralization
///
/// Simple `s` suffix is added. For irregular plurals, use `prefix` to
/// customize the full path.
pub fn build_collection_path(entity: &EntityDef) -> String {
    let api_config = entity.api_config();
    let prefix = api_config.full_path_prefix();
    let entity_path = entity.name_str().to_case(Case::Kebab);

    let path = format!("{}/{}s", prefix, entity_path);
    path.replace("//", "/")
}

/// Builds the item path for an entity (e.g., `/users/{id}`).
///
/// Item paths are used for operations that affect a single entity identified
/// by its primary key: `GET` (get), `PATCH` (update), and `DELETE` (delete).
///
/// # Arguments
///
/// * `entity` - The parsed entity definition
///
/// # Returns
///
/// A path string with the format `/{collection}/{id}`.
///
/// # Path Construction
///
/// ```text
/// build_collection_path()
///         │
///         ▼
///     /api/v1/users
///         │
///         ├─► append "/{id}"
///         │
///         ▼
///     /api/v1/users/{id}
/// ```
///
/// # OpenAPI Path Parameters
///
/// The `{id}` placeholder is an OpenAPI path parameter. When documented:
///
/// ```yaml
/// /users/{id}:
///   get:
///     parameters:
///       - name: id
///         in: path
///         required: true
///         schema:
///           type: string
///           format: uuid
/// ```
///
/// # Examples
///
/// | Entity | Prefix | Version | Result |
/// |--------|--------|---------|--------|
/// | `User` | - | - | `/users/{id}` |
/// | `User` | `api` | `v1` | `/api/v1/users/{id}` |
/// | `BlogPost` | - | - | `/blog-posts/{id}` |
pub fn build_item_path(entity: &EntityDef) -> String {
    let collection = build_collection_path(entity);
    format!("{}/{{id}}", collection)
}

/// Generates the handler function name for a command.
///
/// Command handlers follow the naming pattern `{command}_{entity}` in
/// snake_case, consistent with the CRUD handler naming convention.
///
/// # Arguments
///
/// * `entity` - The parsed entity definition
/// * `cmd` - The command definition
///
/// # Returns
///
/// A `syn::Ident` for the handler function name.
///
/// # Naming Convention
///
/// ```text
/// Command: "Ban"     Entity: "User"
///     │                   │
///     ▼                   ▼
///   "ban"    +   "_"  + "user"
///     │                   │
///     └───────┬───────────┘
///             ▼
///        "ban_user"
/// ```
///
/// # Examples
///
/// | Command | Entity | Result |
/// |---------|--------|--------|
/// | `Ban` | `User` | `ban_user` |
/// | `Activate` | `Account` | `activate_account` |
/// | `SendVerification` | `User` | `send_verification_user` |
///
/// # Usage
///
/// Used when generating command path operations and their operationIds:
///
/// ```rust,ignore
/// let handler = command_handler_name(&entity, &cmd);
/// // handler = "ban_user"
///
/// // In generated code:
/// pub async fn ban_user(...) { ... }
/// ```
#[allow(dead_code)]
pub fn command_handler_name(entity: &EntityDef, cmd: &CommandDef) -> syn::Ident {
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let cmd_snake = cmd.name.to_string().to_case(Case::Snake);
    format_ident!("{}_{}", cmd_snake, entity_snake)
}
