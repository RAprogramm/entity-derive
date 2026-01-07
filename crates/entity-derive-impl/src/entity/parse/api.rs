// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

#![allow(dead_code)] // Methods used by handler generation (#77)

//! API configuration parsing for OpenAPI/utoipa integration.
//!
//! This module handles parsing of `#[entity(api(...))]` attributes for
//! automatic HTTP handler generation with OpenAPI documentation.
//!
//! # Syntax
//!
//! ```rust,ignore
//! #[entity(api(
//!     tag = "Users",                    // OpenAPI tag name (required)
//!     tag_description = "...",          // Tag description (optional)
//!     path_prefix = "/api/v1",          // URL prefix (optional)
//!     security = "bearer",              // Default security scheme (optional)
//!     public = [Register, Login],       // Commands without auth (optional)
//! ))]
//! ```
//!
//! # Generated Output
//!
//! When `api(...)` is present, the macro generates:
//! - Axum handlers with `#[utoipa::path]` annotations
//! - OpenAPI schemas via `#[derive(ToSchema)]`
//! - Router factory function
//! - OpenApi struct for Swagger UI

use syn::Ident;

/// Handler configuration for selective CRUD generation.
///
/// # Syntax
///
/// - `handlers` - enables all handlers
/// - `handlers(create, get, list)` - enables specific handlers
#[derive(Debug, Clone, Default)]
pub struct HandlerConfig {
    /// Generate create handler (POST /collection).
    pub create: bool,
    /// Generate get handler (GET /collection/{id}).
    pub get:    bool,
    /// Generate update handler (PATCH /collection/{id}).
    pub update: bool,
    /// Generate delete handler (DELETE /collection/{id}).
    pub delete: bool,
    /// Generate list handler (GET /collection).
    pub list:   bool
}

impl HandlerConfig {
    /// Create config with all handlers enabled.
    pub fn all() -> Self {
        Self {
            create: true,
            get:    true,
            update: true,
            delete: true,
            list:   true
        }
    }

    /// Check if any handler is enabled.
    pub fn any(&self) -> bool {
        self.create || self.get || self.update || self.delete || self.list
    }
}

/// API configuration parsed from `#[entity(api(...))]`.
///
/// Controls HTTP handler generation and OpenAPI documentation.
#[derive(Debug, Clone, Default)]
pub struct ApiConfig {
    /// OpenAPI tag name for grouping endpoints.
    ///
    /// Required when API generation is enabled.
    /// Example: `"Users"`, `"Products"`, `"Orders"`
    pub tag: Option<String>,

    /// Description for the OpenAPI tag.
    ///
    /// Provides additional context in API documentation.
    pub tag_description: Option<String>,

    /// URL path prefix for all endpoints.
    ///
    /// Example: `"/api/v1"` results in `/api/v1/users`
    pub path_prefix: Option<String>,

    /// Default security scheme for endpoints.
    ///
    /// Supported values:
    /// - `"bearer"` - JWT Bearer token
    /// - `"api_key"` - API key in header
    /// - `"none"` - No authentication
    pub security: Option<String>,

    /// Commands that don't require authentication.
    ///
    /// These endpoints bypass the default security scheme.
    /// Example: `[Register, Login]`
    pub public_commands: Vec<Ident>,

    /// API version string.
    ///
    /// Added to path prefix: `/api/v1` with version `"v1"`
    pub version: Option<String>,

    /// Version in which this API is deprecated.
    ///
    /// Marks all endpoints with `deprecated = true` in OpenAPI.
    pub deprecated_in: Option<String>,

    /// CRUD handlers configuration.
    ///
    /// Controls which handlers to generate:
    /// - `handlers` - all handlers
    /// - `handlers(create, get, list)` - specific handlers only
    pub handlers: HandlerConfig,

    /// OpenAPI info: API title.
    ///
    /// Overrides the default title in OpenAPI spec.
    /// Example: `"User Service API"`
    pub title: Option<String>,

    /// OpenAPI info: API description.
    ///
    /// Full description for the API, supports Markdown.
    /// Example: `"RESTful API for user management"`
    pub description: Option<String>,

    /// OpenAPI info: API version.
    ///
    /// Semantic version string for the API.
    /// Example: `"1.0.0"`
    pub api_version: Option<String>,

    /// OpenAPI info: License name.
    ///
    /// License under which the API is published.
    /// Example: `"MIT"`, `"Apache-2.0"`
    pub license: Option<String>,

    /// OpenAPI info: License URL.
    ///
    /// URL to the license text.
    pub license_url: Option<String>,

    /// OpenAPI info: Contact name.
    ///
    /// Name of the API maintainer or team.
    pub contact_name: Option<String>,

    /// OpenAPI info: Contact email.
    ///
    /// Email for API support inquiries.
    pub contact_email: Option<String>,

    /// OpenAPI info: Contact URL.
    ///
    /// URL to API support or documentation.
    pub contact_url: Option<String>
}

impl ApiConfig {
    /// Check if API generation is enabled.
    ///
    /// Returns `true` if the `api(...)` attribute is present.
    pub fn is_enabled(&self) -> bool {
        self.tag.is_some()
    }

    /// Get the tag name or default to entity name.
    ///
    /// # Arguments
    ///
    /// * `entity_name` - Fallback entity name
    pub fn tag_or_default(&self, entity_name: &str) -> String {
        self.tag.clone().unwrap_or_else(|| entity_name.to_string())
    }

    /// Get the full path prefix including version.
    ///
    /// Combines `path_prefix` and `version` if both are set.
    pub fn full_path_prefix(&self) -> String {
        match (&self.path_prefix, &self.version) {
            (Some(prefix), Some(version)) => {
                format!("{}/{}", prefix.trim_end_matches('/'), version)
            }
            (Some(prefix), None) => prefix.clone(),
            (None, Some(version)) => format!("/{}", version),
            (None, None) => String::new()
        }
    }

    /// Check if a command is public (no auth required).
    ///
    /// # Arguments
    ///
    /// * `command_name` - Command name to check
    pub fn is_public_command(&self, command_name: &str) -> bool {
        self.public_commands.iter().any(|c| c == command_name)
    }

    /// Check if API is marked as deprecated.
    pub fn is_deprecated(&self) -> bool {
        self.deprecated_in.is_some()
    }

    /// Check if any CRUD handler should be generated.
    pub fn has_handlers(&self) -> bool {
        self.handlers.any()
    }

    /// Get handler configuration.
    pub fn handlers(&self) -> &HandlerConfig {
        &self.handlers
    }

    /// Get security scheme for a command.
    ///
    /// Returns `None` for public commands, otherwise the default security.
    ///
    /// # Arguments
    ///
    /// * `command_name` - Command name to check
    pub fn security_for_command(&self, command_name: &str) -> Option<&str> {
        if self.is_public_command(command_name) {
            None
        } else {
            self.security.as_deref()
        }
    }
}

/// Parse `#[entity(api(...))]` attribute.
///
/// Extracts API configuration from the nested attribute.
///
/// # Arguments
///
/// * `meta` - The meta content inside `api(...)`
///
/// # Returns
///
/// Parsed `ApiConfig` or error.
pub fn parse_api_config(meta: &syn::Meta) -> syn::Result<ApiConfig> {
    let mut config = ApiConfig::default();

    let list = match meta {
        syn::Meta::List(list) => list,
        syn::Meta::Path(_) => {
            return Err(syn::Error::new_spanned(
                meta,
                "api attribute requires parameters: api(tag = \"...\")"
            ));
        }
        syn::Meta::NameValue(_) => {
            return Err(syn::Error::new_spanned(
                meta,
                "api attribute must use parentheses: api(tag = \"...\")"
            ));
        }
    };

    list.parse_nested_meta(|nested| {
        let ident = nested
            .path
            .get_ident()
            .ok_or_else(|| syn::Error::new_spanned(&nested.path, "expected identifier"))?;
        let ident_str = ident.to_string();

        match ident_str.as_str() {
            "tag" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.tag = Some(value.value());
            }
            "tag_description" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.tag_description = Some(value.value());
            }
            "path_prefix" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.path_prefix = Some(value.value());
            }
            "security" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.security = Some(value.value());
            }
            "public" => {
                let _: syn::Token![=] = nested.input.parse()?;
                let content;
                syn::bracketed!(content in nested.input);
                let commands =
                    syn::punctuated::Punctuated::<Ident, syn::Token![,]>::parse_terminated(
                        &content
                    )?;
                config.public_commands = commands.into_iter().collect();
            }
            "version" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.version = Some(value.value());
            }
            "deprecated_in" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.deprecated_in = Some(value.value());
            }
            "handlers" => {
                // Support:
                // - `handlers` - all handlers
                // - `handlers = true/false` - all or none
                // - `handlers(create, get, list)` - specific handlers
                if nested.input.peek(syn::Token![=]) {
                    let _: syn::Token![=] = nested.input.parse()?;
                    let value: syn::LitBool = nested.input.parse()?;
                    if value.value() {
                        config.handlers = HandlerConfig::all();
                    }
                } else if nested.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in nested.input);
                    let handlers =
                        syn::punctuated::Punctuated::<Ident, syn::Token![,]>::parse_terminated(
                            &content
                        )?;
                    for handler in handlers {
                        match handler.to_string().as_str() {
                            "create" => config.handlers.create = true,
                            "get" => config.handlers.get = true,
                            "update" => config.handlers.update = true,
                            "delete" => config.handlers.delete = true,
                            "list" => config.handlers.list = true,
                            other => {
                                return Err(syn::Error::new(
                                    handler.span(),
                                    format!(
                                        "unknown handler '{}', expected: create, get, update, delete, list",
                                        other
                                    )
                                ));
                            }
                        }
                    }
                } else {
                    config.handlers = HandlerConfig::all();
                }
            }
            "title" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.title = Some(value.value());
            }
            "description" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.description = Some(value.value());
            }
            "api_version" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.api_version = Some(value.value());
            }
            "license" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.license = Some(value.value());
            }
            "license_url" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.license_url = Some(value.value());
            }
            "contact_name" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.contact_name = Some(value.value());
            }
            "contact_email" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.contact_email = Some(value.value());
            }
            "contact_url" => {
                let value: syn::LitStr = nested.value()?.parse()?;
                config.contact_url = Some(value.value());
            }
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!(
                        "unknown api option '{}', expected: tag, tag_description, path_prefix, \
                         security, public, version, deprecated_in, handlers, title, description, \
                         api_version, license, license_url, contact_name, contact_email, contact_url",
                        ident_str
                    )
                ));
            }
        }

        Ok(())
    })?;

    Ok(config)
}

#[cfg(test)]
mod tests {
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
        let config = parse_test_config(
            r#"api(tag = "Users", handlers(create, get, update, delete, list))"#
        );
        assert!(config.handlers().create);
        assert!(config.handlers().get);
        assert!(config.handlers().update);
        assert!(config.handlers().delete);
        assert!(config.handlers().list);
    }
}
