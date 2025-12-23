//! Mapper generation for the Entity derive macro.
//!
//! Generates `From` implementations between Entity, DTOs, Row, and Insertable.

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EntityDef, SqlLevel};
use crate::utils::fields;

/// Generate all `From` implementations.
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
    let assigns = fields::create_assigns(entity.all_fields(), &create_fields);

    quote! {
        impl From<#create_name> for #entity_name {
            fn from(dto: #create_name) -> Self {
                Self { #(#assigns),* }
            }
        }
    }
}
