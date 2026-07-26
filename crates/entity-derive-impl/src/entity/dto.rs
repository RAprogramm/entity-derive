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
use quote::{format_ident, quote};

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

    let extra_derives = dto_extra_derives();
    let marker = marker::generated();

    quote! {
        #marker
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #extra_derives
        #vis struct #name { #(#field_defs),* }
    }
}

/// Backend-dependent derives for generated DTOs.
///
/// Emitted only for features enabled on the facade at expansion time,
/// so consumer crates never receive `cfg_attr`s for features they do
/// not declare (`unexpected_cfgs`).
fn dto_extra_derives() -> TokenStream {
    let mut derives = TokenStream::new();
    if cfg!(feature = "api") {
        derives.extend(quote! { #[derive(utoipa::ToSchema)] });
    }
    if cfg!(feature = "validate") {
        derives.extend(quote! { #[derive(validator::Validate)] });
    } else if cfg!(feature = "garde") {
        derives.extend(quote! { #[derive(garde::Validate)] });
    }
    derives
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

    if !cfg!(feature = "garde") || cfg!(feature = "validate") {
        return TokenStream::new();
    }
    let tokens: TokenStream = body.parse().expect("garde rules are valid tokens");
    quote! { #[garde(#tokens)] }
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
                    with = "::entity_derive::serde_helpers::double_option"
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

    let version_garde_skip = if cfg!(feature = "garde") && !cfg!(feature = "validate") {
        quote! { #[garde(skip)] }
    } else {
        TokenStream::new()
    };
    let version_field = entity.version_field().map(|f| {
        let vt = f.ty();
        quote! {
            /// Version observed by the caller (optimistic locking).
            ///
            /// The UPDATE only applies when the row still carries this
            /// version; on mismatch the call fails with a conflict.
            #version_garde_skip
            pub expected_version: #vt,
        }
    });

    let extra_derives = dto_extra_derives();
    let marker = marker::generated();
    let builders = generate_update_builders(entity, &name);

    quote! {
        #marker
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        #extra_derives
        #vis struct #name {
            #(#field_defs,)*
            #version_field
        }

        #builders
    }
}

/// Generate chainable setters for the update DTO.
///
/// A patch built from a struct literal has to spell the wrapping out —
/// `Some(value)` for a plain column and `Some(Some(value))` for a
/// nullable one, where `Some(None)` means "write NULL". The setters say
/// the same thing in the caller's words, and struct-literal
/// construction keeps working unchanged.
fn generate_update_builders(entity: &EntityDef, name: &syn::Ident) -> TokenStream {
    let fields = entity.update_fields();
    if fields.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let methods: Vec<TokenStream> = fields
        .iter()
        .flat_map(|f| {
            let field = f.name();
            let setter = format_ident!("set_{}", f.name_str());
            if f.is_option() {
                let inner = f.option_inner_type();
                let clear = format_ident!("clear_{}", f.name_str());
                let set_doc =
                    format!("Set `{}` to the given value.", f.name_str());
                let clear_doc = format!(
                    "Write NULL to `{}`.\n\nLeaving the field untouched keeps the stored value;                      this asks for the column to be cleared.",
                    f.name_str()
                );
                vec![
                    quote! {
                        #[doc = #set_doc]
                        #[must_use]
                        #vis fn #setter(mut self, value: #inner) -> Self {
                            self.#field = Some(Some(value));
                            self
                        }
                    },
                    quote! {
                        #[doc = #clear_doc]
                        #[must_use]
                        #vis fn #clear(mut self) -> Self {
                            self.#field = Some(None);
                            self
                        }
                    },
                ]
            } else {
                let ty = f.ty();
                let set_doc = format!("Set `{}` to the given value.", f.name_str());
                vec![quote! {
                    #[doc = #set_doc]
                    #[must_use]
                    #vis fn #setter(mut self, value: #ty) -> Self {
                        self.#field = Some(value);
                        self
                    }
                }]
            }
        })
        .collect();

    let version_method = entity.version_field().map(|f| {
        let ty = f.ty();
        quote! {
            /// Record the version the caller observed.
            ///
            /// The update applies only while the row still carries it.
            #[must_use]
            #vis fn expecting_version(mut self, version: #ty) -> Self {
                self.expected_version = version;
                self
            }
        }
    });

    quote! {
        impl #name {
            #(#methods)*
            #version_method
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

    let extra_derives_api = if cfg!(feature = "api") {
        quote! { #[derive(utoipa::ToSchema)] }
    } else {
        TokenStream::new()
    };
    let marker = marker::generated();

    quote! {
        #marker
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #extra_derives_api
        #vis struct #name { #(#field_defs),* }
    }
}

#[cfg(all(test, feature = "garde", not(feature = "validate")))]
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
    fn garde_derive_emitted_without_cfg_wrapper() {
        let code = generate(&validated_entity()).to_string();
        assert!(code.contains("derive (garde :: Validate)"));
        assert!(!code.contains("cfg_attr"));
    }
}

#[cfg(all(test, feature = "garde", feature = "validate"))]
mod garde_precedence_tests {
    use syn::DeriveInput;

    use super::*;

    #[test]
    fn validate_wins_over_garde() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[validate(email)]
                pub email: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let code = generate(&entity).to_string();
        assert!(code.contains("validator :: Validate"));
        assert!(!code.contains("garde"));
    }
}
