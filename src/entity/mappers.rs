//! Mapper generation for Entity derive macro.
//!
//! Generates From/Into implementations between Entity, DTOs, Row, and
//! Insertable.

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EntityDef, SqlLevel};

/// Generate all From implementations.
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

/// Generate From<Row> for Entity.
fn generate_row_to_entity(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let row_name = entity.ident_with("", "Row");
    let fields = entity.all_fields();

    let field_assigns: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { #name: row.#name }
        })
        .collect();

    quote! {
        impl From<#row_name> for #entity_name {
            fn from(row: #row_name) -> Self {
                Self {
                    #(#field_assigns),*
                }
            }
        }
    }
}

/// Generate From<Entity> for Insertable and From<&Entity> for Insertable.
fn generate_entity_to_insertable(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let insertable_name = entity.ident_with("Insertable", "");
    let fields = entity.all_fields();

    let field_assigns: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { #name: entity.#name }
        })
        .collect();

    let field_assigns_clone: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { #name: entity.#name.clone() }
        })
        .collect();

    quote! {
        impl From<#entity_name> for #insertable_name {
            fn from(entity: #entity_name) -> Self {
                Self {
                    #(#field_assigns),*
                }
            }
        }

        impl From<&#entity_name> for #insertable_name {
            fn from(entity: &#entity_name) -> Self {
                Self {
                    #(#field_assigns_clone),*
                }
            }
        }
    }
}

/// Generate From<Entity> for Response.
fn generate_entity_to_response(entity: &EntityDef) -> TokenStream {
    let response_fields = entity.response_fields();
    if response_fields.is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let response_name = entity.ident_with("", "Response");

    let field_assigns: Vec<_> = response_fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { #name: entity.#name }
        })
        .collect();

    let field_assigns_clone: Vec<_> = response_fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { #name: entity.#name.clone() }
        })
        .collect();

    quote! {
        impl From<#entity_name> for #response_name {
            fn from(entity: #entity_name) -> Self {
                Self {
                    #(#field_assigns),*
                }
            }
        }

        impl From<&#entity_name> for #response_name {
            fn from(entity: &#entity_name) -> Self {
                Self {
                    #(#field_assigns_clone),*
                }
            }
        }
    }
}

/// Generate From<CreateRequest> for Entity (partial, needs defaults).
fn generate_create_to_entity(entity: &EntityDef) -> TokenStream {
    let create_fields = entity.create_fields();
    if create_fields.is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let create_name = entity.ident_with("Create", "Request");
    let all_fields = entity.all_fields();

    let field_assigns: Vec<_> = all_fields
        .iter()
        .map(|f| {
            let name = f.name();
            let is_in_create = create_fields.iter().any(|cf| cf.name() == name);

            if is_in_create {
                quote! { #name: dto.#name }
            } else if f.is_id {
                // Generate new UUID for id
                quote! { #name: uuid::Uuid::now_v7() }
            } else if f.is_auto {
                // Auto fields get default
                quote! { #name: Default::default() }
            } else {
                // Other fields get default
                quote! { #name: Default::default() }
            }
        })
        .collect();

    quote! {
        impl From<#create_name> for #entity_name {
            fn from(dto: #create_name) -> Self {
                Self {
                    #(#field_assigns),*
                }
            }
        }
    }
}
