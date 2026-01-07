// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Tests for API configuration parsing.

use super::*;

fn parse_test_config(input: &str) -> ApiConfig {
    let meta: syn::Meta = syn::parse_str(input).unwrap();
    parse_api_config(&meta).unwrap()
}

#[test]
fn parse_tag_only() {
    let config = parse_test_config(r#"api(tag = "Users")"#);
    assert_eq!(config.tag, Some("Users".to_string()));
    assert!(config.is_enabled());
}

#[test]
fn parse_full_config() {
    let config = parse_test_config(
        r#"api(
            tag = "Users",
            tag_description = "User management",
            path_prefix = "/api/v1",
            security = "bearer"
        )"#
    );
    assert_eq!(config.tag, Some("Users".to_string()));
    assert_eq!(config.tag_description, Some("User management".to_string()));
    assert_eq!(config.path_prefix, Some("/api/v1".to_string()));
    assert_eq!(config.security, Some("bearer".to_string()));
}

#[test]
fn parse_public_commands() {
    let config = parse_test_config(r#"api(tag = "Users", public = [Register, Login])"#);
    assert_eq!(config.public_commands.len(), 2);
    assert!(config.is_public_command("Register"));
    assert!(config.is_public_command("Login"));
    assert!(!config.is_public_command("Update"));
}

#[test]
fn parse_version() {
    let config = parse_test_config(r#"api(tag = "Users", version = "v2")"#);
    assert_eq!(config.version, Some("v2".to_string()));
}

#[test]
fn parse_deprecated() {
    let config = parse_test_config(r#"api(tag = "Users", deprecated_in = "v2")"#);
    assert!(config.is_deprecated());
}

#[test]
fn full_path_prefix_with_version() {
    let config = ApiConfig {
        path_prefix: Some("/api".to_string()),
        version: Some("v1".to_string()),
        ..Default::default()
    };
    assert_eq!(config.full_path_prefix(), "/api/v1");
}

#[test]
fn full_path_prefix_without_version() {
    let config = ApiConfig {
        path_prefix: Some("/api/v1".to_string()),
        ..Default::default()
    };
    assert_eq!(config.full_path_prefix(), "/api/v1");
}

#[test]
fn full_path_prefix_version_only() {
    let config = ApiConfig {
        version: Some("v1".to_string()),
        ..Default::default()
    };
    assert_eq!(config.full_path_prefix(), "/v1");
}

#[test]
fn security_for_public_command() {
    let config =
        parse_test_config(r#"api(tag = "Users", security = "bearer", public = [Register])"#);
    assert_eq!(config.security_for_command("Update"), Some("bearer"));
    assert_eq!(config.security_for_command("Register"), None);
}

#[test]
fn tag_or_default_uses_tag() {
    let config = parse_test_config(r#"api(tag = "Users")"#);
    assert_eq!(config.tag_or_default("User"), "Users");
}

#[test]
fn tag_or_default_uses_entity_name() {
    let config = ApiConfig::default();
    assert_eq!(config.tag_or_default("User"), "User");
}

#[test]
fn default_config_not_enabled() {
    let config = ApiConfig::default();
    assert!(!config.is_enabled());
}

#[test]
fn parse_trailing_slash_in_prefix() {
    let config = ApiConfig {
        path_prefix: Some("/api/".to_string()),
        version: Some("v1".to_string()),
        ..Default::default()
    };
    assert_eq!(config.full_path_prefix(), "/api/v1");
}

#[test]
fn parse_handlers_flag() {
    let config = parse_test_config(r#"api(tag = "Users", handlers)"#);
    assert!(config.has_handlers());
}

#[test]
fn parse_handlers_true() {
    let config = parse_test_config(r#"api(tag = "Users", handlers = true)"#);
    assert!(config.has_handlers());
}

#[test]
fn parse_handlers_false() {
    let config = parse_test_config(r#"api(tag = "Users", handlers = false)"#);
    assert!(!config.has_handlers());
}

#[test]
fn default_handlers_false() {
    let config = parse_test_config(r#"api(tag = "Users")"#);
    assert!(!config.has_handlers());
}

#[test]
fn parse_handlers_selective() {
    let config = parse_test_config(r#"api(tag = "Users", handlers(create, get, list))"#);
    assert!(config.has_handlers());
    assert!(config.handlers().create);
    assert!(config.handlers().get);
    assert!(!config.handlers().update);
    assert!(!config.handlers().delete);
    assert!(config.handlers().list);
}

#[test]
fn parse_handlers_single() {
    let config = parse_test_config(r#"api(tag = "Users", handlers(get))"#);
    assert!(config.has_handlers());
    assert!(!config.handlers().create);
    assert!(config.handlers().get);
    assert!(!config.handlers().update);
    assert!(!config.handlers().delete);
    assert!(!config.handlers().list);
}

#[test]
fn parse_handlers_all_explicit() {
    let config =
        parse_test_config(r#"api(tag = "Users", handlers(create, get, update, delete, list))"#);
    assert!(config.handlers().create);
    assert!(config.handlers().get);
    assert!(config.handlers().update);
    assert!(config.handlers().delete);
    assert!(config.handlers().list);
}
