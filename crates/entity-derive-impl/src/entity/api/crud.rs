// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! CRUD handler generation with utoipa annotations.
//!
//! Generates production-ready REST handlers with:
//! - OpenAPI documentation via `#[utoipa::path]`
//! - Cookie/Bearer authentication via `security` attribute
//! - Proper error responses using `masterror::ErrorResponse`
//! - Standard HTTP status codes and error handling
//!
//! # Generated Handlers
//!
//! | Operation | HTTP Method | Path | Status Codes |
//! |-----------|-------------|------|--------------|
//! | Create | POST | `/{entities}` | 201, 400, 401, 500 |
//! | Get | GET | `/{entities}/{id}` | 200, 401, 404, 500 |
//! | Update | PATCH | `/{entities}/{id}` | 200, 400, 401, 404, 500 |
//! | Delete | DELETE | `/{entities}/{id}` | 204, 401, 404, 500 |
//! | List | GET | `/{entities}` | 200, 401, 500 |
//!
//! # Security
//!
//! When `security = "cookie"` or `security = "bearer"` is specified,
//! handlers require authentication and use `Claims` extractor.
//!
//! # Example
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users", api(tag = "Users", security = "cookie", handlers))]
//! pub struct User { /* ... */ }
//!
//! // Generated handler with auth:
//! #[utoipa::path(
//!     post,
//!     path = "/users",
//!     tag = "Users",
//!     request_body(content = CreateUserRequest, description = "User data to create"),
//!     responses(
//!         (status = 201, description = "User created successfully", body = UserResponse),
//!         (status = 400, description = "Invalid request data", body = ErrorResponse),
//!         (status = 401, description = "Authentication required", body = ErrorResponse),
//!         (status = 500, description = "Internal server error", body = ErrorResponse)
//!     ),
//!     security(("cookieAuth" = []))
//! )]
//! pub async fn create_user<R>(
//!     _claims: Claims,
//!     State(repo): State<Arc<R>>,
//!     Json(dto): Json<CreateUserRequest>,
//! ) -> AppResult<(StatusCode, Json<UserResponse>)>
//! where
//!     R: UserRepository + 'static,
//! { /* ... */ }
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::entity::parse::EntityDef;

/// Generate CRUD handler functions based on enabled handlers.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.api_config().has_handlers() {
        return TokenStream::new();
    }

    let handlers = entity.api_config().handlers();

    let create = if handlers.create {
        generate_create_handler(entity)
    } else {
        TokenStream::new()
    };
    let get = if handlers.get {
        generate_get_handler(entity)
    } else {
        TokenStream::new()
    };
    let update = if handlers.update {
        generate_update_handler(entity)
    } else {
        TokenStream::new()
    };
    let delete = if handlers.delete {
        generate_delete_handler(entity)
    } else {
        TokenStream::new()
    };
    let list = if handlers.list {
        generate_list_handler(entity)
    } else {
        TokenStream::new()
    };

    quote! {
        #create
        #get
        #update
        #delete
        #list
    }
}

/// Generate the create handler.
fn generate_create_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("create_{}", entity_name_str.to_case(Case::Snake));
    let create_dto = entity.ident_with("Create", "Request");
    let response_dto = entity.ident_with("", "Response");

    let path = build_collection_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let request_body_desc = format!("Data for creating a new {}", entity_name);
    let success_desc = format!("{} created successfully", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                post,
                path = #path,
                tag = #tag,
                request_body(content = #create_dto, description = #request_body_desc),
                responses(
                    (status = 201, description = #success_desc, body = #response_dto),
                    (status = 400, description = "Invalid request data"),
                    (status = 401, description = "Authentication required"),
                    (status = 500, description = "Internal server error")
                ),
                #security_attr
                #deprecated_attr
            )]
        }
    } else {
        quote! {
            #[utoipa::path(
                post,
                path = #path,
                tag = #tag,
                request_body(content = #create_dto, description = #request_body_desc),
                responses(
                    (status = 201, description = #success_desc, body = #response_dto),
                    (status = 400, description = "Invalid request data"),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "Create a new {}.\n\n\
         # Responses\n\n\
         - `201 Created` - {} created successfully\n\
         - `400 Bad Request` - Invalid request data\n\
         {}\
         - `500 Internal Server Error` - Database or server error",
        entity_name,
        entity_name,
        if has_security {
            "- `401 Unauthorized` - Authentication required\n"
        } else {
            ""
        }
    );

    quote! {
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Json(dto): axum::extract::Json<#create_dto>,
        ) -> masterror::AppResult<(axum::http::StatusCode, axum::response::Json<#response_dto>)>
        where
            R: #repo_trait + 'static,
        {
            let entity = repo
                .create(dto)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?;
            Ok((axum::http::StatusCode::CREATED, axum::response::Json(#response_dto::from(entity))))
        }
    }
}

/// Generate the get handler.
fn generate_get_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let id_field = entity.id_field();
    let id_type = &id_field.ty;
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("get_{}", entity_name_str.to_case(Case::Snake));
    let response_dto = entity.ident_with("", "Response");

    let path = build_item_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let id_desc = format!("{} unique identifier", entity_name);
    let success_desc = format!("{} found", entity_name);
    let not_found_desc = format!("{} not found", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                get,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                responses(
                    (status = 200, description = #success_desc, body = #response_dto),
                    (status = 401, description = "Authentication required"),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                ),
                #security_attr
                #deprecated_attr
            )]
        }
    } else {
        quote! {
            #[utoipa::path(
                get,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                responses(
                    (status = 200, description = #success_desc, body = #response_dto),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "Get {} by ID.\n\n\
         # Responses\n\n\
         - `200 OK` - {} found\n\
         {}\
         - `404 Not Found` - {} not found\n\
         - `500 Internal Server Error` - Database or server error",
        entity_name,
        entity_name,
        if has_security {
            "- `401 Unauthorized` - Authentication required\n"
        } else {
            ""
        },
        entity_name
    );

    let not_found_msg = format!("{} not found", entity_name);

    quote! {
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Path(id): axum::extract::Path<#id_type>,
        ) -> masterror::AppResult<axum::response::Json<#response_dto>>
        where
            R: #repo_trait + 'static,
        {
            let entity = repo
                .find_by_id(id)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?
                .ok_or_else(|| masterror::AppError::not_found(#not_found_msg))?;
            Ok(axum::response::Json(#response_dto::from(entity)))
        }
    }
}

/// Generate the update handler.
fn generate_update_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let id_field = entity.id_field();
    let id_type = &id_field.ty;
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("update_{}", entity_name_str.to_case(Case::Snake));
    let update_dto = entity.ident_with("Update", "Request");
    let response_dto = entity.ident_with("", "Response");

    let path = build_item_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let id_desc = format!("{} unique identifier", entity_name);
    let request_body_desc = format!("Fields to update for {}", entity_name);
    let success_desc = format!("{} updated successfully", entity_name);
    let not_found_desc = format!("{} not found", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                patch,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                request_body(content = #update_dto, description = #request_body_desc),
                responses(
                    (status = 200, description = #success_desc, body = #response_dto),
                    (status = 400, description = "Invalid request data"),
                    (status = 401, description = "Authentication required"),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                ),
                #security_attr
                #deprecated_attr
            )]
        }
    } else {
        quote! {
            #[utoipa::path(
                patch,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                request_body(content = #update_dto, description = #request_body_desc),
                responses(
                    (status = 200, description = #success_desc, body = #response_dto),
                    (status = 400, description = "Invalid request data"),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "Update {} by ID.\n\n\
         # Responses\n\n\
         - `200 OK` - {} updated successfully\n\
         - `400 Bad Request` - Invalid request data\n\
         {}\
         - `404 Not Found` - {} not found\n\
         - `500 Internal Server Error` - Database or server error",
        entity_name,
        entity_name,
        if has_security {
            "- `401 Unauthorized` - Authentication required\n"
        } else {
            ""
        },
        entity_name
    );

    quote! {
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Path(id): axum::extract::Path<#id_type>,
            axum::extract::Json(dto): axum::extract::Json<#update_dto>,
        ) -> masterror::AppResult<axum::response::Json<#response_dto>>
        where
            R: #repo_trait + 'static,
        {
            let entity = repo
                .update(id, dto)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?;
            Ok(axum::response::Json(#response_dto::from(entity)))
        }
    }
}

/// Generate the delete handler.
fn generate_delete_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let id_field = entity.id_field();
    let id_type = &id_field.ty;
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("delete_{}", entity_name_str.to_case(Case::Snake));

    let path = build_item_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let id_desc = format!("{} unique identifier", entity_name);
    let success_desc = format!("{} deleted successfully", entity_name);
    let not_found_desc = format!("{} not found", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                delete,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                responses(
                    (status = 204, description = #success_desc),
                    (status = 401, description = "Authentication required"),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                ),
                #security_attr
                #deprecated_attr
            )]
        }
    } else {
        quote! {
            #[utoipa::path(
                delete,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                responses(
                    (status = 204, description = #success_desc),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "Delete {} by ID.\n\n\
         # Responses\n\n\
         - `204 No Content` - {} deleted successfully\n\
         {}\
         - `404 Not Found` - {} not found\n\
         - `500 Internal Server Error` - Database or server error",
        entity_name,
        entity_name,
        if has_security {
            "- `401 Unauthorized` - Authentication required\n"
        } else {
            ""
        },
        entity_name
    );

    let not_found_msg = format!("{} not found", entity_name);

    quote! {
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Path(id): axum::extract::Path<#id_type>,
        ) -> masterror::AppResult<axum::http::StatusCode>
        where
            R: #repo_trait + 'static,
        {
            let deleted = repo
                .delete(id)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?;
            if deleted {
                Ok(axum::http::StatusCode::NO_CONTENT)
            } else {
                Err(masterror::AppError::not_found(#not_found_msg))
            }
        }
    }
}

/// Generate the list handler.
fn generate_list_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("list_{}", entity_name_str.to_case(Case::Snake));
    let response_dto = entity.ident_with("", "Response");

    let path = build_collection_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let success_desc = format!("List of {} entities", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                get,
                path = #path,
                tag = #tag,
                params(
                    ("limit" = Option<i64>, Query, description = "Maximum number of items to return (default: 100)"),
                    ("offset" = Option<i64>, Query, description = "Number of items to skip for pagination")
                ),
                responses(
                    (status = 200, description = #success_desc, body = Vec<#response_dto>),
                    (status = 401, description = "Authentication required"),
                    (status = 500, description = "Internal server error")
                ),
                #security_attr
                #deprecated_attr
            )]
        }
    } else {
        quote! {
            #[utoipa::path(
                get,
                path = #path,
                tag = #tag,
                params(
                    ("limit" = Option<i64>, Query, description = "Maximum number of items to return (default: 100)"),
                    ("offset" = Option<i64>, Query, description = "Number of items to skip for pagination")
                ),
                responses(
                    (status = 200, description = #success_desc, body = Vec<#response_dto>),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "List {} entities with pagination.\n\n\
         # Query Parameters\n\n\
         - `limit` - Maximum number of items to return (default: 100)\n\
         - `offset` - Number of items to skip for pagination\n\n\
         # Responses\n\n\
         - `200 OK` - List of {} entities\n\
         {}\
         - `500 Internal Server Error` - Database or server error",
        entity_name,
        entity_name,
        if has_security {
            "- `401 Unauthorized` - Authentication required\n"
        } else {
            ""
        }
    );

    quote! {
        /// Pagination query parameters.
        #[derive(Debug, Clone, serde::Deserialize, utoipa::IntoParams)]
        #vis struct PaginationQuery {
            /// Maximum number of items to return.
            #[serde(default = "default_limit")]
            pub limit: i64,
            /// Number of items to skip for pagination.
            #[serde(default)]
            pub offset: i64,
        }

        fn default_limit() -> i64 { 100 }

        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
        ) -> masterror::AppResult<axum::response::Json<Vec<#response_dto>>>
        where
            R: #repo_trait + 'static,
        {
            let entities = repo
                .list(pagination.limit, pagination.offset)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?;
            let responses: Vec<#response_dto> = entities.into_iter().map(#response_dto::from).collect();
            Ok(axum::response::Json(responses))
        }
    }
}

/// Build the collection path (e.g., `/api/v1/users`).
fn build_collection_path(entity: &EntityDef) -> String {
    let api_config = entity.api_config();
    let prefix = api_config.full_path_prefix();
    let entity_path = entity.name_str().to_case(Case::Kebab);

    let path = format!("{}/{}s", prefix, entity_path);
    path.replace("//", "/")
}

/// Build the item path (e.g., `/api/v1/users/{id}`).
fn build_item_path(entity: &EntityDef) -> String {
    let collection = build_collection_path(entity);
    format!("{}/{{id}}", collection)
}

/// Build security attribute for a handler.
///
/// Returns the appropriate security scheme based on the `security` option:
/// - `"cookie"` -> `security(("cookieAuth" = []))`
/// - `"bearer"` -> `security(("bearerAuth" = []))`
/// - `"api_key"` -> `security(("apiKey" = []))`
fn build_security_attr(entity: &EntityDef) -> TokenStream {
    let api_config = entity.api_config();

    if let Some(security) = &api_config.security {
        let security_name = match security.as_str() {
            "cookie" => "cookieAuth",
            "bearer" => "bearerAuth",
            "api_key" => "apiKey",
            _ => "cookieAuth"
        };
        quote! { security((#security_name = [])) }
    } else {
        TokenStream::new()
    }
}

/// Build deprecated attribute if API is deprecated.
fn build_deprecated_attr(entity: &EntityDef) -> TokenStream {
    if entity.api_config().is_deprecated() {
        quote! { , deprecated = true }
    } else {
        TokenStream::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entity() -> EntityDef {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        };
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn collection_path_format() {
        let entity = create_test_entity();
        let path = build_collection_path(&entity);
        assert_eq!(path, "/users");
    }

    #[test]
    fn item_path_format() {
        let entity = create_test_entity();
        let path = build_item_path(&entity);
        assert_eq!(path, "/users/{id}");
    }

    #[test]
    fn generates_handlers_when_enabled() {
        let entity = create_test_entity();
        let tokens = generate(&entity);
        let output = tokens.to_string();
        assert!(output.contains("create_user"));
        assert!(output.contains("get_user"));
        assert!(output.contains("update_user"));
        assert!(output.contains("delete_user"));
        assert!(output.contains("list_user"));
    }

    #[test]
    fn no_handlers_when_disabled() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users"))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let tokens = generate(&entity);
        assert!(tokens.is_empty());
    }
}
