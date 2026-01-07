// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Get handler generation.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::helpers::{build_deprecated_attr, build_item_path, build_security_attr};
use crate::entity::parse::EntityDef;

/// Generate the get handler.
pub fn generate_get_handler(entity: &EntityDef) -> TokenStream {
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
