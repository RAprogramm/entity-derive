// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI struct generation for utoipa 5.x.
//!
//! Generates a struct that implements `utoipa::OpenApi` for Swagger UI
//! integration, with security schemes and paths added via the `Modify` trait.
//!
//! # Generated Code
//!
//! For `User` entity with handlers and security:
//!
//! ```rust,ignore
//! /// OpenAPI modifier for User entity.
//! struct UserApiModifier;
//!
//! impl utoipa::Modify for UserApiModifier {
//!     fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
//!         // Add security schemes
//!         // Add CRUD paths with documentation
//!     }
//! }
//!
//! /// OpenAPI documentation for User entity endpoints.
//! #[derive(utoipa::OpenApi)]
//! #[openapi(
//!     components(schemas(UserResponse, CreateUserRequest, UpdateUserRequest)),
//!     modifiers(&UserApiModifier),
//!     tags((name = "Users", description = "User management"))
//! )]
//! pub struct UserApi;
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::entity::parse::{CommandDef, EntityDef};

/// Generate the OpenAPI struct with modifier.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let has_crud = entity.api_config().has_handlers();
    let has_commands = !entity.command_defs().is_empty();

    if !has_crud && !has_commands {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let api_config = entity.api_config();

    let api_struct = format_ident!("{}Api", entity_name);
    let modifier_struct = format_ident!("{}ApiModifier", entity_name);

    let tag = api_config.tag_or_default(&entity.name_str());
    let tag_description = api_config
        .tag_description
        .clone()
        .or_else(|| entity.doc().map(String::from))
        .unwrap_or_else(|| format!("{} management", entity_name));

    let schema_types = generate_all_schema_types(entity);
    let modifier_impl = generate_modifier(entity, &modifier_struct);

    let doc = format!(
        "OpenAPI documentation for {} entity endpoints.\n\n\
         # Usage\n\n\
         ```rust,ignore\n\
         use utoipa::OpenApi;\n\
         let openapi = {}::openapi();\n\
         ```",
        entity_name, api_struct
    );

    quote! {
        #modifier_impl

        #[doc = #doc]
        #[derive(utoipa::OpenApi)]
        #[openapi(
            components(schemas(#schema_types)),
            modifiers(&#modifier_struct),
            tags((name = #tag, description = #tag_description))
        )]
        #vis struct #api_struct;
    }
}

/// Generate all schema types (DTOs, commands).
fn generate_all_schema_types(entity: &EntityDef) -> TokenStream {
    let entity_name_str = entity.name_str();
    let mut types: Vec<TokenStream> = Vec::new();

    // CRUD DTOs
    if entity.api_config().has_handlers() {
        let response = entity.ident_with("", "Response");
        let create = entity.ident_with("Create", "Request");
        let update = entity.ident_with("Update", "Request");
        types.push(quote! { #response });
        types.push(quote! { #create });
        types.push(quote! { #update });
    }

    // Command structs
    for cmd in entity.command_defs() {
        let cmd_struct = cmd.struct_name(&entity_name_str);
        types.push(quote! { #cmd_struct });
    }

    quote! { #(#types),* }
}

/// Generate the modifier struct with Modify implementation.
///
/// This adds security schemes, common schemas, CRUD paths, and info to the
/// OpenAPI spec.
fn generate_modifier(entity: &EntityDef, modifier_name: &syn::Ident) -> TokenStream {
    let entity_name = entity.name();
    let api_config = entity.api_config();

    let info_code = generate_info_code(entity);
    let security_code = generate_security_code(api_config.security.as_deref());
    let common_schemas_code = if api_config.has_handlers() {
        generate_common_schemas_code()
    } else {
        TokenStream::new()
    };
    let paths_code = if api_config.has_handlers() {
        generate_paths_code(entity)
    } else {
        TokenStream::new()
    };

    let doc = format!("OpenAPI modifier for {} entity.", entity_name);

    quote! {
        #[doc = #doc]
        struct #modifier_name;

        impl utoipa::Modify for #modifier_name {
            fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
                use utoipa::openapi::*;

                #info_code
                #security_code
                #common_schemas_code
                #paths_code
            }
        }
    }
}

/// Generate code to configure OpenAPI info section.
///
/// Sets title, description, version, license, and contact information.
fn generate_info_code(entity: &EntityDef) -> TokenStream {
    let api_config = entity.api_config();

    // Title: use configured or generate default
    let title_code = if let Some(ref title) = api_config.title {
        quote! { openapi.info.title = #title.to_string(); }
    } else {
        TokenStream::new()
    };

    // Description
    let description_code = if let Some(ref description) = api_config.description {
        quote! { openapi.info.description = Some(#description.to_string()); }
    } else if let Some(doc) = entity.doc() {
        // Use entity doc comment if no description configured
        quote! { openapi.info.description = Some(#doc.to_string()); }
    } else {
        TokenStream::new()
    };

    // API Version
    let version_code = if let Some(ref version) = api_config.api_version {
        quote! { openapi.info.version = #version.to_string(); }
    } else {
        TokenStream::new()
    };

    // License
    let license_code = match (&api_config.license, &api_config.license_url) {
        (Some(name), Some(url)) => {
            quote! {
                openapi.info.license = Some(
                    info::LicenseBuilder::new()
                        .name(#name)
                        .url(Some(#url))
                        .build()
                );
            }
        }
        (Some(name), None) => {
            quote! {
                openapi.info.license = Some(
                    info::LicenseBuilder::new()
                        .name(#name)
                        .build()
                );
            }
        }
        _ => TokenStream::new()
    };

    // Contact
    let has_contact = api_config.contact_name.is_some()
        || api_config.contact_email.is_some()
        || api_config.contact_url.is_some();

    let contact_code = if has_contact {
        let name = api_config.contact_name.as_deref().unwrap_or("");
        let email = api_config.contact_email.as_deref();
        let url = api_config.contact_url.as_deref();

        let email_setter = if let Some(e) = email {
            quote! { .email(Some(#e)) }
        } else {
            TokenStream::new()
        };

        let url_setter = if let Some(u) = url {
            quote! { .url(Some(#u)) }
        } else {
            TokenStream::new()
        };

        quote! {
            openapi.info.contact = Some(
                info::ContactBuilder::new()
                    .name(Some(#name))
                    #email_setter
                    #url_setter
                    .build()
            );
        }
    } else {
        TokenStream::new()
    };

    // Deprecated flag
    let deprecated_code = if api_config.is_deprecated() {
        let version = api_config.deprecated_in.as_deref().unwrap_or("unknown");
        let msg = format!("Deprecated since {}", version);
        quote! {
            // Mark in description that API is deprecated
            if let Some(ref desc) = openapi.info.description {
                openapi.info.description = Some(format!("**DEPRECATED**: {}\n\n{}", #msg, desc));
            } else {
                openapi.info.description = Some(format!("**DEPRECATED**: {}", #msg));
            }
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #title_code
        #description_code
        #version_code
        #license_code
        #contact_code
        #deprecated_code
    }
}

/// Generate common schemas (ErrorResponse, PaginationQuery) for the OpenAPI
/// spec.
fn generate_common_schemas_code() -> TokenStream {
    quote! {
        // Add ErrorResponse schema for error responses
        if let Some(components) = openapi.components.as_mut() {
            // ErrorResponse schema (RFC 7807 Problem Details)
            let error_schema = schema::ObjectBuilder::new()
                .schema_type(schema::Type::Object)
                .title(Some("ErrorResponse"))
                .description(Some("Error response following RFC 7807 Problem Details"))
                .property("type", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::String)
                    .description(Some("A URI reference that identifies the problem type"))
                    .example(Some(serde_json::json!("https://errors.example.com/not-found")))
                    .build())
                .required("type")
                .property("title", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::String)
                    .description(Some("A short, human-readable summary of the problem"))
                    .example(Some(serde_json::json!("Resource not found")))
                    .build())
                .required("title")
                .property("status", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::Integer)
                    .description(Some("HTTP status code"))
                    .example(Some(serde_json::json!(404)))
                    .build())
                .required("status")
                .property("detail", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::String)
                    .description(Some("A human-readable explanation specific to this occurrence"))
                    .example(Some(serde_json::json!("User with ID '123' was not found")))
                    .build())
                .property("code", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::String)
                    .description(Some("Application-specific error code"))
                    .example(Some(serde_json::json!("NOT_FOUND")))
                    .build())
                .build();

            components.schemas.insert("ErrorResponse".to_string(), error_schema.into());

            // PaginationQuery schema
            let pagination_schema = schema::ObjectBuilder::new()
                .schema_type(schema::Type::Object)
                .title(Some("PaginationQuery"))
                .description(Some("Query parameters for paginated list endpoints"))
                .property("limit", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::Integer)
                    .description(Some("Maximum number of items to return"))
                    .default(Some(serde_json::json!(100)))
                    .minimum(Some(1.0))
                    .maximum(Some(1000.0))
                    .build())
                .property("offset", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::Integer)
                    .description(Some("Number of items to skip for pagination"))
                    .default(Some(serde_json::json!(0)))
                    .minimum(Some(0.0))
                    .build())
                .build();

            components.schemas.insert("PaginationQuery".to_string(), pagination_schema.into());
        }
    }
}

/// Generate security scheme code for the Modify implementation.
fn generate_security_code(security: Option<&str>) -> TokenStream {
    let Some(security) = security else {
        return TokenStream::new();
    };

    let (scheme_name, scheme_impl) = match security {
        "cookie" => (
            "cookieAuth",
            quote! {
                security::SecurityScheme::ApiKey(
                    security::ApiKey::Cookie(
                        security::ApiKeyValue::with_description(
                            "token",
                            "JWT token stored in HTTP-only cookie"
                        )
                    )
                )
            }
        ),
        "bearer" => (
            "bearerAuth",
            quote! {
                security::SecurityScheme::Http(
                    security::HttpBuilder::new()
                        .scheme(security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT token in Authorization header"))
                        .build()
                )
            }
        ),
        "api_key" => (
            "apiKey",
            quote! {
                security::SecurityScheme::ApiKey(
                    security::ApiKey::Header(
                        security::ApiKeyValue::with_description(
                            "X-API-Key",
                            "API key for service-to-service authentication"
                        )
                    )
                )
            }
        ),
        _ => return TokenStream::new()
    };

    quote! {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(#scheme_name, #scheme_impl);
        }
    }
}

/// Generate code to add CRUD paths to OpenAPI.
///
/// Only generates paths for enabled handlers based on `HandlerConfig`.
fn generate_paths_code(entity: &EntityDef) -> TokenStream {
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

    // Schema names for $ref - just the name, not full path
    let response_ref = response_schema.to_string();
    let create_ref = create_schema.to_string();
    let update_ref = update_schema.to_string();

    // Security requirement
    let security_req = if let Some(security) = &api_config.security {
        let scheme_name = match security.as_str() {
            "cookie" => "cookieAuth",
            "bearer" => "bearerAuth",
            "api_key" => "apiKey",
            _ => "cookieAuth"
        };
        quote! {
            Some(vec![security::SecurityRequirement::new::<_, _, &str>(#scheme_name, [])])
        }
    } else {
        quote! { None }
    };

    // ID parameter type (only needed for item paths)
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

    // Common code: error helper, params, security
    let common_code = quote! {
        // Helper to build error response with ErrorResponse schema
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

        // Security requirements
        let security_req: Option<Vec<security::SecurityRequirement>> = #security_req;
    };

    // ID parameter (only if needed)
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

    // CREATE handler
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

    // LIST handler
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

    // GET handler
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

    // UPDATE handler
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

    // DELETE handler
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

/// Build the collection path (e.g., `/users`).
fn build_collection_path(entity: &EntityDef) -> String {
    let api_config = entity.api_config();
    let prefix = api_config.full_path_prefix();
    let entity_path = entity.name_str().to_case(Case::Kebab);

    let path = format!("{}/{}s", prefix, entity_path);
    path.replace("//", "/")
}

/// Build the item path (e.g., `/users/{id}`).
fn build_item_path(entity: &EntityDef) -> String {
    let collection = build_collection_path(entity);
    format!("{}/{{id}}", collection)
}

/// Get command handler function name.
#[allow(dead_code)]
fn command_handler_name(entity: &EntityDef, cmd: &CommandDef) -> syn::Ident {
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let cmd_snake = cmd.name.to_string().to_case(Case::Snake);
    format_ident!("{}_{}", cmd_snake, entity_snake)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_crud_only() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let tokens = generate(&entity);
        let output = tokens.to_string();
        assert!(output.contains("UserApi"));
        assert!(output.contains("UserApiModifier"));
        assert!(output.contains("UserResponse"));
        assert!(output.contains("CreateUserRequest"));
    }

    #[test]
    fn generate_with_security() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", security = "bearer", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let tokens = generate(&entity);
        let output = tokens.to_string();
        assert!(output.contains("UserApiModifier"));
        assert!(output.contains("bearerAuth"));
    }

    #[test]
    fn generate_cookie_security() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", security = "cookie", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let tokens = generate(&entity);
        let output = tokens.to_string();
        assert!(output.contains("cookieAuth"));
    }

    #[test]
    fn no_api_when_disabled() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let tokens = generate(&entity);
        assert!(tokens.is_empty());
    }

    #[test]
    fn collection_path_format() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let path = build_collection_path(&entity);
        assert_eq!(path, "/users");
    }

    #[test]
    fn item_path_format() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let path = build_item_path(&entity);
        assert_eq!(path, "/users/{id}");
    }
}
