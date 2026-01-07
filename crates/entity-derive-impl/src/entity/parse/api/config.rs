// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! API configuration types.

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
