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

#[test]
fn lookup_fields_returns_unique_and_index() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users")]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique)]
            pub email: String,
            #[field(create, response)]
            #[column(index)]
            pub status: String,
            #[field(create, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let fields = entity.lookup_fields();
    assert_eq!(fields.len(), 2);

    let names: Vec<String> = fields.iter().map(|f| f.name_str()).collect();
    assert!(names.contains(&"email".to_string()));
    assert!(names.contains(&"status".to_string()));
    assert!(!names.contains(&"name".to_string()));
}

#[test]
fn lookup_fields_only_unique() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users")]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique)]
            pub email: String,
            #[field(create, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let fields = entity.lookup_fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name_str(), "email");
}

#[test]
fn lookup_fields_only_index() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "posts")]
        pub struct Post {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(index)]
            pub slug: String,
            #[field(create, response)]
            pub title: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let fields = entity.lookup_fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name_str(), "slug");
}

#[test]
fn lookup_fields_none_returns_empty() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users")]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let fields = entity.lookup_fields();
    assert!(fields.is_empty());
}

#[test]
fn lookup_fields_both_unique_and_index_on_same_field() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "products")]
        pub struct Product {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique, index)]
            pub sku: String,
            #[field(create, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let fields = entity.lookup_fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name_str(), "sku");
    assert!(fields[0].column.unique);
    assert!(fields[0].column.index.is_some());
}

#[test]
fn upsert_valid_unique_column_parses() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users", upsert(conflict = "email"))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique)]
            pub email: String,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let upsert = entity.upsert.expect("upsert must be parsed");
    assert_eq!(upsert.conflict_columns(), vec!["email"]);
}

#[test]
fn upsert_valid_id_conflict_parses() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users", upsert(conflict = "id"))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    assert!(EntityDef::from_derive_input(&input).is_ok());
}

#[test]
fn upsert_valid_unique_index_composite_parses() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(
            table = "members",
            unique_index(tenant_id, email),
            upsert(conflict = "tenant_id, email")
        )]
        pub struct Member {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            pub tenant_id: uuid::Uuid,
            #[field(create, response)]
            pub email: String,
            #[field(create, update, response)]
            pub role: String,
        }
    };
    assert!(EntityDef::from_derive_input(&input).is_ok());
}

#[test]
fn upsert_empty_conflict_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users", upsert(conflict = " , "))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("at least one conflict column"));
}

#[test]
fn upsert_without_create_fields_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "counters", upsert(conflict = "id"))]
        pub struct Counter {
            #[id]
            pub id: uuid::Uuid,
            #[field(response)]
            #[auto]
            pub value: i64,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("#[field(create)]"));
}

#[test]
fn upsert_non_full_returning_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users", returning = "id", upsert(conflict = "email"))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique)]
            pub email: String,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("returning = \"full\""));
}

#[test]
fn upsert_unknown_column_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users", upsert(conflict = "missing"))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique)]
            pub email: String,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("does not match any entity column"));
}

#[test]
fn upsert_non_unique_column_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users", upsert(conflict = "email"))]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            pub email: String,
            #[field(create, update, response)]
            pub name: String,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("no uniqueness guarantee"));
}

#[test]
fn upsert_composite_without_unique_index_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "members", upsert(conflict = "tenant_id, email"))]
        pub struct Member {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            pub tenant_id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique)]
            pub email: String,
            #[field(create, update, response)]
            pub role: String,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("no uniqueness guarantee"));
}

#[test]
fn upsert_update_without_updatable_columns_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "subscriptions", upsert(conflict = "email"))]
        pub struct Subscription {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique)]
            pub email: String,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("action = \"nothing\""));
}

#[test]
fn upsert_nothing_without_updatable_columns_parses() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "subscriptions", upsert(conflict = "email", action = "nothing"))]
        pub struct Subscription {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            #[column(unique)]
            pub email: String,
        }
    };
    assert!(EntityDef::from_derive_input(&input).is_ok());
}

#[test]
fn upsert_unique_index_order_insensitive() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(
            table = "members",
            unique_index(email, tenant_id),
            upsert(conflict = "tenant_id, email")
        )]
        pub struct Member {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, response)]
            pub tenant_id: uuid::Uuid,
            #[field(create, response)]
            pub email: String,
            #[field(create, update, response)]
            pub role: String,
        }
    };
    assert!(EntityDef::from_derive_input(&input).is_ok());
}

#[test]
fn owner_field_accessor_returns_marked_field() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "orders")]
        pub struct Order {
            #[id]
            pub id: uuid::Uuid,
            #[owner]
            pub user_id: uuid::Uuid,
            #[field(create, response)]
            pub note: String,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    assert_eq!(entity.owner_field().unwrap().name_str(), "user_id");
}

#[test]
fn owner_duplicate_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "orders")]
        pub struct Order {
            #[id]
            pub id: uuid::Uuid,
            #[owner]
            pub user_id: uuid::Uuid,
            #[owner]
            pub tenant_id: uuid::Uuid,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("at most one #[owner]"));
}

#[test]
fn owner_on_id_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "orders")]
        pub struct Order {
            #[id]
            #[owner]
            pub id: uuid::Uuid,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("cannot be combined with #[id]"));
}

#[test]
fn owner_absent_returns_none() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "users")]
        pub struct User {
            #[id]
            pub id: uuid::Uuid,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    assert!(entity.owner_field().is_none());
}

#[test]
fn version_field_parsed_with_auto_default() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "orders")]
        pub struct Order {
            #[id]
            pub id: uuid::Uuid,
            #[field(create, update, response)]
            pub note: String,
            #[version]
            #[field(response)]
            #[auto]
            pub version: i32,
        }
    };
    let entity = EntityDef::from_derive_input(&input).unwrap();
    let field = entity.version_field().unwrap();
    assert_eq!(field.name_str(), "version");
    assert_eq!(field.column.default.as_deref(), Some("0"));
}

#[test]
fn version_duplicate_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "orders")]
        pub struct Order {
            #[id]
            pub id: uuid::Uuid,
            #[version]
            pub v1: i32,
            #[version]
            pub v2: i32,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("at most one #[version]"));
}

#[test]
fn version_non_integer_rejected() {
    let input: DeriveInput = syn::parse_quote! {
        #[entity(table = "orders")]
        pub struct Order {
            #[id]
            pub id: uuid::Uuid,
            #[version]
            pub version: String,
        }
    };
    let err = EntityDef::from_derive_input(&input).unwrap_err();
    assert!(err.to_string().contains("integer field"));
}
