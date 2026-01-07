// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! List handler generation.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::helpers::{build_collection_path, build_deprecated_attr, build_security_attr};
use crate::entity::parse::EntityDef;

/// Generate the list handler.
pub fn generate_list_handler(entity: &EntityDef) -> TokenStream {
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
