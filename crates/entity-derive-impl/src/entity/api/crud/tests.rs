// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Tests for CRUD handler generation.

use super::*;

fn create_test_entity() -> EntityDef {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users", handlers))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    EntityDef::from_derive_input(&input).unwrap()
}

#[test]
fn collection_path_format() {
    let entity = create_test_entity();
    let path = build_collection_path(&entity);
    assert_eq!(path, "/users");
}

#[test]
fn item_path_format() {
    let entity = create_test_entity();
    let path = build_item_path(&entity);
    assert_eq!(path, "/users/{id}");
}

#[test]
fn generates_handlers_when_enabled() {
    let entity = create_test_entity();
    let tokens = generate(&entity);
    let output = tokens.to_string();
    assert!(output.contains("create_user"));
    assert!(output.contains("get_user"));
    assert!(output.contains("update_user"));
    assert!(output.contains("delete_user"));
    assert!(output.contains("list_user"));
}

#[test]
fn no_handlers_when_disabled() {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users"))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let tokens = generate(&entity);
    assert!(tokens.is_empty());
}
