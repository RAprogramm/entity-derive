// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Data Transfer Object (DTO) generation.
//!
//! This module generates three DTO structs for API layer separation:
//!
//! | Struct | Purpose | Fields |
//! |--------|---------|--------|
//! | `Create{Name}Request` | Entity creation | `#[field(create)]` fields |
//! | `Update{Name}Request` | Partial updates | `#[field(update)]` fields (wrapped in `Option`) |
//! | `{Name}Response` | API responses | `#[field(response)]` + `#[id]` fields |
//!
//! # Derive Macros
//!
//! All DTOs automatically derive:
//! - `Debug`, `Clone` — standard traits
//! - `serde::Serialize`, `serde::Deserialize` — JSON serialization
//!
//! # Feature Flags
//!
//! - `api` — adds `utoipa::ToSchema` for `OpenAPI` documentation
//! - `validate` — adds `validator::Validate` for input validation
//!
//! # Field Selection
//!
//! Fields are included based on attributes:
//!
//! ```rust,ignore
//! #[field(create)]           // → CreateRequest only
//! #[field(update)]           // → UpdateRequest only
//! #[field(response)]         // → Response only
//! #[field(create, response)] // → CreateRequest + Response
//! #[field(skip)]             // → excluded from all DTOs
//! #[id]                      // → always in Response
//! #[auto]                    // → excluded from Create/Update
//! ```

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EntityDef, FieldDef};
use crate::utils::marker;

/// Generates all DTO structs for the entity.
///
/// Returns a combined `TokenStream` containing `CreateRequest`,
/// `UpdateRequest`, and `Response` struct definitions.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let create = generate_create_dto(entity);
    let update = generate_update_dto(entity);
    let response = generate_response_dto(entity);

    quote! { #create #update #response }
}

fn generate_create_dto(entity: &EntityDef) -> TokenStream {
    let fields = entity.create_fields();
    if fields.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let name = entity.ident_with("Create", "Request");
    let field_defs = fields.iter().map(|f| {
        let n = f.name();
        let t = f.ty();
        let garde = garde_attr(f, 0);
        quote! {
            #garde
            pub #n: #t
        }
    });

    let marker = marker::generated();

    quote! {
        #marker
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #[cfg_attr(feature = "validate", derive(validator::Validate))]
        #[cfg_attr(all(feature = "garde", not(feature = "validate")), derive(garde::Validate))]
        #vis struct #name { #(#field_defs),* }
    }
}

/// Build the `#[garde(...)]` attribute for a DTO field.
///
/// Translates the typed validation constraints into garde rules;
/// fields without constraints get `#[garde(skip)]` (garde requires an
/// annotation on every field). `option_depth` wraps the rules in
/// `inner(...)` per `Option` layer so Update DTO wrappers validate the
/// contained value.
fn garde_attr(field: &FieldDef, option_depth: usize) -> TokenStream {
    let v = &field.validation;
    let mut rules: Vec<String> = Vec::new();

    match (v.min_length, v.max_length) {
        (Some(min), Some(max)) => rules.push(format!("length(min = {min}, max = {max})")),
        (Some(min), None) => rules.push(format!("length(min = {min})")),
        (None, Some(max)) => rules.push(format!("length(max = {max})")),
        (None, None) => {}
    }
    match (v.minimum, v.maximum) {
        (Some(min), Some(max)) => rules.push(format!("range(min = {min}, max = {max})")),
        (Some(min), None) => rules.push(format!("range(min = {min})")),
        (None, Some(max)) => rules.push(format!("range(max = {max})")),
        (None, None) => {}
    }
    if v.email {
        rules.push("email".to_string());
    }
    if v.url {
        rules.push("url".to_string());
    }
    if let Some(pattern) = &v.pattern {
        rules.push(format!("pattern(\"{pattern}\")"));
    }

    let body = if rules.is_empty() {
        "skip".to_string()
    } else {
        let mut inner = rules.join(", ");
        for _ in 0..option_depth {
            inner = format!("inner({inner})");
        }
        inner
    };

    let tokens: TokenStream = body.parse().expect("garde rules are valid tokens");
    quote! { #[cfg_attr(all(feature = "garde", not(feature = "validate")), garde(#tokens))] }
}

fn generate_update_dto(entity: &EntityDef) -> TokenStream {
    let fields = entity.update_fields();
    if fields.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let name = entity.ident_with("Update", "Request");
    let field_defs = fields.iter().map(|f| {
        let n = f.name();
        let t = f.ty();
        if f.is_option() {
            let garde = garde_attr(f, 2);
            quote! {
                #[serde(
                    default,
                    skip_serializing_if = "Option::is_none",
                    with = "::entity_core::serde_helpers::double_option"
                )]
                #garde
                pub #n: Option<#t>
            }
        } else {
            let garde = garde_attr(f, 1);
            quote! {
                #garde
                pub #n: Option<#t>
            }
        }
    });

    let version_field = entity.version_field().map(|f| {
        let vt = f.ty();
        quote! {
            /// Version observed by the caller (optimistic locking).
            ///
            /// The UPDATE only applies when the row still carries this
            /// version; on mismatch the call fails with a conflict.
            #[cfg_attr(all(feature = "garde", not(feature = "validate")), garde(skip))]
            pub expected_version: #vt,
        }
    });

    let marker = marker::generated();

    quote! {
        #marker
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #[cfg_attr(feature = "validate", derive(validator::Validate))]
        #[cfg_attr(all(feature = "garde", not(feature = "validate")), derive(garde::Validate))]
        #vis struct #name {
            #(#field_defs,)*
            #version_field
        }

    }
}

fn generate_response_dto(entity: &EntityDef) -> TokenStream {
    let fields = entity.response_fields();
    if fields.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let name = entity.ident_with("", "Response");
    let field_defs = fields.iter().map(|f| {
        let n = f.name();
        let t = f.ty();
        quote! { pub #n: #t }
    });

    let marker = marker::generated();

    quote! {
        #marker
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
        #vis struct #name { #(#field_defs),* }
    }
}

#[cfg(test)]
mod garde_tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::*;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    fn validated_entity() -> EntityDef {
        parse_entity(quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                #[validate(length(min = 1, max = 64))]
                pub name: String,
                #[field(create, response)]
                #[validate(email)]
                pub email: String,
                #[field(create, update, response)]
                pub bio: Option<String>,
            }
        })
    }

    #[test]
    fn create_dto_translates_constraints() {
        let code = generate(&validated_entity()).to_string();
        assert!(code.contains("garde (length (min = 1 , max = 64))"));
        assert!(code.contains("garde (email)"));
        assert!(code.contains("derive (garde :: Validate)"));
    }

    #[test]
    fn unconstrained_fields_get_skip() {
        let code = generate(&validated_entity()).to_string();
        assert!(code.contains("garde (skip)"));
    }

    #[test]
    fn update_dto_wraps_rules_in_inner() {
        let code = generate(&validated_entity()).to_string();
        assert!(code.contains("inner (length (min = 1 , max = 64))"));
    }

    #[test]
    fn validate_takes_precedence_when_both_enabled() {
        let code = generate(&validated_entity()).to_string();
        assert!(code.contains("all (feature = \"garde\" , not (feature = \"validate\"))"));
    }
}
