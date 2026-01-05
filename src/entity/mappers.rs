// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
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
use crate::utils::fields;

/// Generates all `From` implementations for the entity.
///
/// Combines all mapper generations into a single `TokenStream`.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let row_to_entity = generate_row_to_entity(entity);
    let entity_to_insertable = generate_entity_to_insertable(entity);
    let entity_to_response = generate_entity_to_response(entity);
    let create_to_entity = generate_create_to_entity(entity);

    quote! {
        #row_to_entity
        #entity_to_insertable
        #entity_to_response
        #create_to_entity
    }
}

fn generate_row_to_entity(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let row_name = entity.ident_with("", "Row");
    let assigns = fields::assigns(entity.all_fields(), "row");

    quote! {
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

    quote! {
        impl From<#entity_name> for #insertable_name {
            fn from(entity: #entity_name) -> Self {
                Self { #(#assigns),* }
            }
        }

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

    quote! {
        impl From<#entity_name> for #response_name {
            fn from(entity: #entity_name) -> Self {
                Self { #(#assigns),* }
            }
        }

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

    quote! {
        impl From<#create_name> for #entity_name {
            fn from(dto: #create_name) -> Self {
                Self { #(#assigns),* }
            }
        }
    }
}
