// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Tests for entity parsing.
//!
//! This module contains comprehensive tests for `EntityDef` parsing from
//! `#[entity(...)]` attributes. Tests cover all configuration options,
//! error handling, and edge cases.
//!
//! # Test Categories
//!
//! | Category | Tests | Coverage |
//! |----------|-------|----------|
//! | Defaults | `default_error_type_is_sqlx_error` | Default values |
//! | Accessors | `entity_def_error_type_accessor` | Method correctness |
//! | API Config | `entity_def_with_api`, `*_full_api_config` | API parsing |
//! | Security | `entity_def_api_with_public_commands` | Security overrides |
//! | No API | `entity_def_without_api` | API disabled |
//!
//! # Test Methodology
//!
//! Tests use `syn::parse_quote!` to create struct definitions with attributes,
//! then verify the parsed `EntityDef` fields match expectations:
//!
//! ```rust,ignore
//! let input: DeriveInput = syn::parse_quote! {
//!     #[entity(table = "users")]
//!     pub struct User {
//!         #[id]
//!         pub id: Uuid,
//!     }
//! };
//! let entity = EntityDef::from_derive_input(&input).unwrap();
//! assert!(!entity.has_api());
//! ```
//!
//! # API Configuration Tests
//!
//! Tests verify correct parsing of nested `api(...)` configuration:
//!
//! | Test | Configuration | Verified |
//! |------|---------------|----------|
//! | `entity_def_with_api` | `api(tag = "Users")` | Tag parsing |
//! | `entity_def_with_full_api_config` | All options | Full configuration |
//! | `entity_def_api_with_public_commands` | `public = [...]` | Security per command |

use syn::DeriveInput;

use super::{EntityDef, attrs::default_error_type};

#[test]
fn default_error_type_is_sqlx_error() {
    let path = default_error_type();
    let path_str = quote::quote!(#path).to_string();
    assert!(path_str.contains("sqlx"));
    assert!(path_str.contains("Error"));
}

#[test]
fn entity_def_error_type_accessor() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users")]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let error_path = entity.error_type();
    let path_str = quote::quote!(#error_path).to_string();
    assert!(path_str.contains("sqlx"));
}

#[test]
fn entity_def_without_api() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users")]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    assert!(!entity.has_api());
}

#[test]
fn entity_def_with_api() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users", api(tag = "Users"))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    assert!(entity.has_api());
    assert_eq!(entity.api_config().tag, Some("Users".to_string()));
}

#[test]
fn entity_def_with_full_api_config() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(
            table = "users",
            api(
                tag = "Users",
                tag_description = "User management",
                path_prefix = "/api/v1",
                security = "bearer"
            )
        )]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    assert!(entity.has_api());
    let config = entity.api_config();
    assert_eq!(config.tag, Some("Users".to_string()));
    assert_eq!(config.tag_description, Some("User management".to_string()));
    assert_eq!(config.path_prefix, Some("/api/v1".to_string()));
    assert_eq!(config.security, Some("bearer".to_string()));
}

#[test]
fn entity_def_api_with_public_commands() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(
            table = "users",
            api(tag = "Users", security = "bearer", public = [Register, Login])
        )]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let config = entity.api_config();
    assert!(config.is_public_command("Register"));
    assert!(config.is_public_command("Login"));
    assert!(!config.is_public_command("Update"));
    assert_eq!(config.security_for_command("Register"), None);
    assert_eq!(config.security_for_command("Update"), Some("bearer"));
}
