// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Type mapper generation for entity conversions.
//!
//! Generates `From` trait implementations for converting between all generated
//! types. These mappers enable seamless data flow through application layers.
//!
//! # Generated Implementations
//!
//! For an entity `User`, the following conversions are generated:
//!
//! | From | To | Purpose |
//! |------|----|---------|
//! | `UserRow` | `User` | DB query result → Domain entity |
//! | `User` | `InsertableUser` | Domain entity → INSERT data |
//! | `&User` | `InsertableUser` | Borrowed entity → INSERT (clones) |
//! | `User` | `UserResponse` | Domain entity → API response |
//! | `&User` | `UserResponse` | Borrowed entity → Response (clones) |
//! | `CreateUserRequest` | `User` | Create DTO → New entity |
//!
//! # Data Flow
//!
//! ```text
//! Create Flow:
//!   CreateUserRequest → User → InsertableUser → DB INSERT
//!                         ↓
//!                    UserResponse → API
//!
//! Read Flow:
//!   DB SELECT → UserRow → User → UserResponse → API
//! ```
//!
//! # Field Handling
//!
//! Each conversion handles fields differently:
//!
//! ## `CreateUserRequest → User`
//!
//! - `#[field(create)]` fields: Copied from DTO
//! - `#[id]` fields: Auto-generated UUID (v7 or v4)
//! - `#[auto]` fields: `Default::default()`
//! - Other fields: `Default::default()`
//!
//! ## `User → UserResponse`
//!
//! - Only `#[field(response)]` and `#[id]` fields are included
//! - `#[field(skip)]` fields are excluded
//!
//! # Conditional Generation
//!
//! | Mapper | Condition |
//! |--------|-----------|
//! | `Row → Entity` | `sql != "none"` |
//! | `Entity → Insertable` | `sql != "none"` |
//! | `Entity → Response` | Has response fields |
//! | `CreateRequest → Entity` | Has create fields |

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EntityDef, SqlLevel};
use crate::utils::{fields, marker};

/// Generates all `From` implementations for the entity.
///
/// Combines all mapper generations into a single `TokenStream`.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let embed_checks = generate_embed_drift_checks(entity);

    let row_to_entity = generate_row_to_entity(entity);
    let entity_to_insertable = generate_entity_to_insertable(entity);
    let entity_to_response = generate_entity_to_response(entity);
    let create_to_entity = generate_create_to_entity(entity);

    quote! {
        #embed_checks
        #row_to_entity
        #entity_to_insertable
        #entity_to_response
        #create_to_entity
    }
}

/// Compile-time shape checks for `#[embed(...)]` declarations.
///
/// Destructures each embedded struct against the declared subfields:
/// a missing, extra, renamed or retyped field in the real struct fails
/// the build instead of silently corrupting the column mapping.
fn generate_embed_drift_checks(entity: &EntityDef) -> TokenStream {
    let checks: Vec<TokenStream> = entity
        .embed_parents()
        .iter()
        .map(|field| {
            let ty = &field.ty;
            let embed = field
                .embed
                .as_ref()
                .expect("embed_parents returns only embed fields");
            let names: Vec<&syn::Ident> = embed.subfields.iter().map(|(n, _)| n).collect();
            let bindings: Vec<TokenStream> = embed
                .subfields
                .iter()
                .map(|(n, t)| quote! { let _: #t = #n; })
                .collect();
            quote! {
                #[allow(dead_code)]
                fn __assert_embed_shape(value: #ty) {
                    let #ty { #(#names),* } = value;
                    #(#bindings)*
                }
            }
        })
        .collect();

    if checks.is_empty() {
        return TokenStream::new();
    }

    quote! {
        const _: () = {
            #(#checks)*
        };
    }
}

fn generate_row_to_entity(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let row_name = entity.ident_with("", "Row");
    let assigns = fields::row_assigns(entity.all_fields(), "row");
    let marker = marker::generated();

    quote! {
        #marker
        impl From<#row_name> for #entity_name {
            fn from(row: #row_name) -> Self {
                Self { #(#assigns),* }
            }
        }
    }
}

fn generate_entity_to_insertable(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let insertable_name = entity.ident_with("Insertable", "");
    let assigns = fields::assigns(entity.all_fields(), "entity");
    let assigns_clone = fields::assigns_clone(entity.all_fields(), "entity");
    let marker = marker::generated();

    quote! {
        #marker
        impl From<#entity_name> for #insertable_name {
            fn from(entity: #entity_name) -> Self {
                Self { #(#assigns),* }
            }
        }

        #marker
        impl From<&#entity_name> for #insertable_name {
            fn from(entity: &#entity_name) -> Self {
                Self { #(#assigns_clone),* }
            }
        }
    }
}

fn generate_entity_to_response(entity: &EntityDef) -> TokenStream {
    let response_fields = entity.response_fields();
    if response_fields.is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let response_name = entity.ident_with("", "Response");
    let assigns = fields::assigns_from_refs(&response_fields, "entity");
    let assigns_clone = fields::assigns_clone_from_refs(&response_fields, "entity");
    let marker = marker::generated();

    quote! {
        #marker
        impl From<#entity_name> for #response_name {
            fn from(entity: #entity_name) -> Self {
                Self { #(#assigns),* }
            }
        }

        #marker
        impl From<&#entity_name> for #response_name {
            fn from(entity: &#entity_name) -> Self {
                Self { #(#assigns_clone),* }
            }
        }
    }
}

fn generate_create_to_entity(entity: &EntityDef) -> TokenStream {
    let create_fields = entity.create_fields();
    if create_fields.is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let create_name = entity.ident_with("Create", "Request");
    let assigns = fields::create_assigns(entity.all_fields(), &create_fields, entity.uuid);
    let marker = marker::generated();

    quote! {
        #marker
        impl From<#create_name> for #entity_name {
            fn from(dto: #create_name) -> Self {
                Self { #(#assigns),* }
            }
        }
    }
}

#[cfg(test)]
mod embed_tests {
    use syn::DeriveInput;

    use super::*;

    fn embedded_entity() -> EntityDef {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "products")]
            pub struct Product {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                #[embed(prefix = "price_", fields(amount_cents: i64, currency: String))]
                pub price: Money,
                #[field(create, response)]
                pub name: String,
            }
        };
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn synthetic_columns_expand_after_parent() {
        let entity = embedded_entity();
        let names: Vec<String> = entity
            .column_fields()
            .iter()
            .map(|f| f.name_str())
            .collect();
        assert_eq!(
            names,
            vec!["id", "price_amount_cents", "price_currency", "name"]
        );
    }

    #[test]
    fn row_to_entity_reconstructs_parent() {
        let code = generate(&embedded_entity()).to_string();
        assert!(code.contains("price : Money"));
        assert!(code.contains("amount_cents : row . price_amount_cents"));
        assert!(code.contains("currency : row . price_currency"));
    }

    #[test]
    fn entity_to_insertable_flattens_parent() {
        let code = generate(&embedded_entity()).to_string();
        assert!(code.contains("price_amount_cents : entity . price . amount_cents . clone ()"));
    }

    #[test]
    fn drift_check_destructures_declared_shape() {
        let code = generate(&embedded_entity()).to_string();
        assert!(code.contains("__assert_embed_shape"));
        assert!(code.contains("let Money { amount_cents , currency }"));
    }

    #[test]
    fn embed_column_collision_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "products")]
            pub struct Product {
                #[id]
                pub id: uuid::Uuid,
                #[embed(prefix = "", fields(name: String))]
                pub price: Money,
                #[field(create, response)]
                pub name: String,
            }
        };
        let err = EntityDef::from_derive_input(&input).unwrap_err();
        assert!(err.to_string().contains("collides"));
    }

    #[test]
    fn embed_option_parent_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "products")]
            pub struct Product {
                #[id]
                pub id: uuid::Uuid,
                #[embed(prefix = "price_", fields(amount: i64))]
                pub price: Option<Money>,
            }
        };
        let err = EntityDef::from_derive_input(&input).unwrap_err();
        assert!(err.to_string().contains("Option"));
    }
}
