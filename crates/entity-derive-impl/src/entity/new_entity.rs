// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! New entity struct generation for aggregate roots.
//!
//! Generates `New{Name}` structs and `From<New{Name}> for {Name}`
//! implementations for aggregate root entities.
//!
//! # Aggregate Root Pattern
//!
//! When `#[entity(aggregate_root)]` is enabled, the entity becomes an
//! aggregate root in the DDD sense. The macro generates:
//!
//! - **`New{Name}` struct** — a create-only DTO without the ID field
//! - **`From<New{Name}> for {Name}`** — converts new data to entity with
//!   auto-generated UUID for the ID
//!
//! # Generated Code
//!
//! For an `Order` aggregate root:
//!
//! ```rust,ignore
//! pub struct NewOrder {
//!     pub buyer_id: Uuid,
//!     pub seller_id: Uuid,
//! }
//!
//! impl From<NewOrder> for Order {
//!     fn from(new: NewOrder) -> Self {
//!         Self {
//!             id: uuid::Uuid::new_v4(),
//!             buyer_id: new.buyer_id,
//!             seller_id: new.seller_id,
//!         }
//!     }
//! }
//! ```
//!
//! # Field Selection
//!
//! `New{Name}` fields include:
//!
//! - `#[field(create)]` fields (excluding `#[id]` and `#[auto]`)
//!
//! # Exclusions
//!
//! - `id` field is never included in `New{Name}` (auto-generated)
//! - `#[auto]` fields are excluded
//! - `#[field(skip)]` fields are excluded
//! - `#[field(update)]`-only fields are excluded
//! - `#[field(response)]`-only fields are excluded

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::entity::parse::{EntityDef, SqlLevel};

/// Generates `New{Name}` struct and `From<New{Name}> for {Name}` for
/// aggregate root entities.
///
/// Returns an empty `TokenStream` if `aggregate_root` is not enabled
/// or if `sql = "none"`.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.is_aggregate_root() || entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let new_struct = generate_new_struct(entity);
    let from_impl = generate_from_impl(entity);

    quote! {
        #new_struct
        #from_impl
    }
}

/// Generate the `New{Name}` struct for the aggregate root entity.
fn generate_new_struct(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let new_name = entity.ident_with("New", "");

    let fields = get_new_fields(entity);
    let field_defs = fields.iter().map(|(name, ty)| {
        quote! { pub #name: #ty }
    });

    let marker = crate::utils::marker::generated();
    let api_derive = if cfg!(feature = "api") {
        quote! { #[derive(utoipa::ToSchema)] }
    } else {
        TokenStream::new()
    };

    quote! {
        #marker
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #api_derive
        #vis struct #new_name {
            #(#field_defs),*
        }
    }
}

/// Generate `From<New{Name}> for {Name}` for the aggregate root.
fn generate_from_impl(entity: &EntityDef) -> TokenStream {
    let entity_name = entity.name();
    let new_name = entity.ident_with("New", "");
    let id_field = entity.id_field();
    let id_name = id_field.name();

    let new_fields = get_new_fields(entity);
    let new_field_set: std::collections::HashSet<&Ident> =
        new_fields.iter().map(|(name, _)| name).collect();

    let mut assignments: Vec<TokenStream> = Vec::new();
    assignments.push(quote! { #id_name: uuid::Uuid::new_v4() });

    for (name, _) in &new_fields {
        assignments.push(quote! { #name: new.#name });
    }

    for field in entity.all_fields() {
        if !field.is_id() && !new_field_set.contains(field.name()) {
            let name = field.name();
            assignments.push(quote! { #name: Default::default() });
        }
    }

    let marker = crate::utils::marker::generated();

    quote! {
        #marker
        impl From<#new_name> for #entity_name {
            fn from(new: #new_name) -> Self {
                Self {
                    #(#assignments),*
                }
            }
        }
    }
}

/// Get fields for the `New{Name}` struct.
///
/// Returns a vector of `(field_name, field_type)` tuples.
/// Includes `#[field(create)]` fields excluding `#[id]` and `#[auto]`.
fn get_new_fields(entity: &EntityDef) -> Vec<(Ident, syn::Type)> {
    let mut fields = Vec::new();

    for field in entity.create_fields() {
        let name = field.name().clone();
        let ty = field.ty().clone();
        fields.push((name, ty));
    }

    fields
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    use super::*;
    use crate::entity::parse::EntityDef;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = parse_quote!(#tokens);
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn new_fields_excludes_id() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders", aggregate_root)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create)]
                pub buyer_id: uuid::Uuid,
            }
        });

        let fields = get_new_fields(&entity);
        let names: Vec<String> = fields.iter().map(|(n, _)| n.to_string()).collect();
        assert!(!names.contains(&"id".to_string()));
        assert!(names.contains(&"buyer_id".to_string()));
    }

    #[test]
    fn new_fields_excludes_auto() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders", aggregate_root)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create)]
                pub buyer_id: uuid::Uuid,
                #[field(response)]
                #[auto]
                pub created_at: chrono::DateTime<chrono::Utc>,
            }
        });

        let fields = get_new_fields(&entity);
        let names: Vec<String> = fields.iter().map(|(n, _)| n.to_string()).collect();
        assert!(!names.contains(&"created_at".to_string()));
    }

    #[test]
    fn new_struct_no_id_field() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders", aggregate_root)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create)]
                pub buyer_id: uuid::Uuid,
            }
        });

        let ts = generate_new_struct(&entity);
        let code = ts.to_string();
        assert!(!code.contains("id:"));
        assert!(code.contains("buyer_id"));
    }

    #[test]
    fn from_impl_generates_uuid() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders", aggregate_root)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create)]
                pub buyer_id: uuid::Uuid,
            }
        });

        let ts = generate_from_impl(&entity);
        let code = ts.to_string();
        assert!(code.contains("new_v4"));
    }

    #[test]
    fn generate_returns_empty_when_not_aggregate_root() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders")]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create)]
                pub buyer_id: uuid::Uuid,
            }
        });

        let ts = generate(&entity);
        assert!(ts.is_empty());
    }

    #[test]
    fn generate_returns_empty_when_sql_none() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders", aggregate_root, sql = "none")]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create)]
                pub buyer_id: uuid::Uuid,
            }
        });

        let ts = generate(&entity);
        assert!(ts.is_empty());
    }

    #[test]
    fn new_struct_includes_create_fields() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders", aggregate_root)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create)]
                pub buyer_id: uuid::Uuid,
                #[field(create)]
                pub seller_id: uuid::Uuid,
            }
        });

        let ts = generate_new_struct(&entity);
        let code = ts.to_string();
        assert!(code.contains("buyer_id"));
        assert!(code.contains("seller_id"));
    }

    #[test]
    fn from_impl_copies_fields() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "orders", aggregate_root)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create)]
                pub buyer_id: uuid::Uuid,
            }
        });

        let ts = generate_from_impl(&entity);
        let code = ts.to_string();
        assert!(code.contains("buyer_id"));
        assert!(code.contains("new"));
    }
}
