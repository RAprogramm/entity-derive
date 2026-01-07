// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Tests for OpenAPI generation.

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
