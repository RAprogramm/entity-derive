// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Tests for OpenAPI generation.
//!
//! This module contains unit tests for the OpenAPI code generation
//! functionality. Tests verify that the generated OpenAPI structs, modifiers,
//! and schemas are correct for various entity configurations.
//!
//! # Test Categories
//!
//! | Category | Tests | Purpose |
//! |----------|-------|---------|
//! | Basic | `generate_crud_only` | Verify struct generation |
//! | Security | `generate_with_security`, `generate_cookie_security` | Auth schemes |
//! | Disabled | `no_api_when_disabled` | No output when API disabled |
//! | Paths | `collection_path_format`, `item_path_format` | URL patterns |
//! | Handlers | `selective_handlers_*` | Conditional schema generation |
//!
//! # Test Methodology
//!
//! Tests use `syn::parse_quote!` to create entity definitions from attribute
//! syntax, then verify the generated `TokenStream` contains expected
//! identifiers.
//!
//! ```rust,ignore
//! let input: syn::DeriveInput = syn::parse_quote! {
//!     #[entity(table = "users", api(handlers))]
//!     pub struct User { ... }
//! };
//! let entity = EntityDef::from_derive_input(&input).unwrap();
//! let tokens = generate(&entity);
//! assert!(tokens.to_string().contains("UserApi"));
//! ```

use super::*;

#[test]
fn generate_crud_only() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", handlers))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let tokens = generate(&entity);
    let output = tokens.to_string();
    assert!(output.contains("UserApi"));
    assert!(output.contains("UserApiModifier"));
    assert!(output.contains("UserResponse"));
    assert!(output.contains("CreateUserRequest"));
}

#[test]
fn generate_with_security() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", security = "bearer", handlers))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let tokens = generate(&entity);
    let output = tokens.to_string();
    assert!(output.contains("UserApiModifier"));
    assert!(output.contains("bearerAuth"));
}

#[test]
fn generate_cookie_security() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", security = "cookie", handlers))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let tokens = generate(&entity);
    let output = tokens.to_string();
    assert!(output.contains("cookieAuth"));
}

#[test]
fn no_api_when_disabled() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users")]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let tokens = generate(&entity);
    assert!(tokens.is_empty());
}

#[test]
fn collection_path_format() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", handlers))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let path = build_collection_path(&entity);
    assert_eq!(path, "/users");
}

#[test]
fn item_path_format() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", handlers))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let path = build_item_path(&entity);
    assert_eq!(path, "/users/{id}");
}

#[test]
fn selective_handlers_schemas_get_list_only() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", handlers(get, list)))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let tokens = generate(&entity);
    let output = tokens.to_string();
    assert!(output.contains("UserResponse"));
    assert!(!output.contains("CreateUserRequest"));
    assert!(!output.contains("UpdateUserRequest"));
}

#[test]
fn selective_handlers_schemas_create_only() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", handlers(create)))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let tokens = generate(&entity);
    let output = tokens.to_string();
    assert!(output.contains("UserResponse"));
    assert!(output.contains("CreateUserRequest"));
    assert!(!output.contains("UpdateUserRequest"));
}

#[test]
fn selective_handlers_all_schemas() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", handlers(create, update)))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let tokens = generate(&entity);
    let output = tokens.to_string();
    assert!(output.contains("UserResponse"));
    assert!(output.contains("CreateUserRequest"));
    assert!(output.contains("UpdateUserRequest"));
}
