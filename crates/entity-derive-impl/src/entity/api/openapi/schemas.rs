// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI schema generation.
//!
//! Generates schema types for DTOs and common schemas like ErrorResponse
//! and PaginationQuery.

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityDef;

/// Generate all schema types (DTOs, commands).
///
/// Only registers schemas for enabled handlers to keep OpenAPI spec clean.
pub fn generate_all_schema_types(entity: &EntityDef) -> TokenStream {
    let entity_name_str = entity.name_str();
    let mut types: Vec<TokenStream> = Vec::new();

    let handlers = entity.api_config().handlers();
    if handlers.any() {
        let response = entity.ident_with("", "Response");
        types.push(quote! { #response });

        if handlers.create {
            let create = entity.ident_with("Create", "Request");
            types.push(quote! { #create });
        }

        if handlers.update {
            let update = entity.ident_with("Update", "Request");
            types.push(quote! { #update });
        }
    }

    for cmd in entity.command_defs() {
        let cmd_struct = cmd.struct_name(&entity_name_str);
        types.push(quote! { #cmd_struct });
    }

    quote! { #(#types),* }
}

/// Generate common schemas (ErrorResponse, PaginationQuery) for the OpenAPI
/// spec.
pub fn generate_common_schemas_code() -> TokenStream {
    quote! {
        if let Some(components) = openapi.components.as_mut() {
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
