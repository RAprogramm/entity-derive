// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI security scheme generation.
//!
//! This module generates security scheme definitions for the OpenAPI
//! specification. Security schemes define how API endpoints are protected
//! and how clients should authenticate.
//!
//! # Supported Security Types
//!
//! The macro supports three authentication mechanisms:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                   Security Schemes                              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  1. Bearer Token (JWT)                                          │
//! │     ├─► Scheme name: "bearerAuth"                               │
//! │     ├─► Type: HTTP Bearer                                       │
//! │     ├─► Header: Authorization: Bearer <token>                   │
//! │     └─► Format: JWT                                             │
//! │                                                                 │
//! │  2. Cookie Authentication                                       │
//! │     ├─► Scheme name: "cookieAuth"                               │
//! │     ├─► Type: API Key (Cookie)                                  │
//! │     ├─► Cookie name: "token"                                    │
//! │     └─► Note: HTTP-only for XSS protection                      │
//! │                                                                 │
//! │  3. API Key                                                     │
//! │     ├─► Scheme name: "apiKey"                                   │
//! │     ├─► Type: API Key (Header)                                  │
//! │     ├─► Header: X-API-Key: <key>                                │
//! │     └─► Use case: Service-to-service auth                       │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Configuration
//!
//! Security type is set via the `security` attribute:
//!
//! ```rust,ignore
//! #[entity(
//!     table = "users",
//!     api(
//!         security = "bearer",  // or "cookie", "api_key"
//!         handlers
//!     )
//! )]
//! pub struct User { ... }
//! ```
//!
//! # Generated Code Examples
//!
//! ## Bearer Token
//!
//! ```rust,ignore
//! components.add_security_scheme("bearerAuth",
//!     security::SecurityScheme::Http(
//!         security::HttpBuilder::new()
//!             .scheme(security::HttpAuthScheme::Bearer)
//!             .bearer_format("JWT")
//!             .description(Some("JWT token in Authorization header"))
//!             .build()
//!     )
//! );
//! ```
//!
//! ## Cookie Authentication
//!
//! ```rust,ignore
//! components.add_security_scheme("cookieAuth",
//!     security::SecurityScheme::ApiKey(
//!         security::ApiKey::Cookie(
//!             security::ApiKeyValue::with_description(
//!                 "token",
//!                 "JWT token stored in HTTP-only cookie"
//!             )
//!         )
//!     )
//! );
//! ```
//!
//! ## API Key
//!
//! ```rust,ignore
//! components.add_security_scheme("apiKey",
//!     security::SecurityScheme::ApiKey(
//!         security::ApiKey::Header(
//!             security::ApiKeyValue::with_description(
//!                 "X-API-Key",
//!                 "API key for service-to-service authentication"
//!             )
//!         )
//!     )
//! );
//! ```
//!
//! # Swagger UI Integration
//!
//! When a security scheme is configured, Swagger UI displays:
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │  🔒 Authorize                                │
//! │  ────────────────────────────────────────────│
//! │  bearerAuth (http, Bearer)                   │
//! │  JWT token in Authorization header           │
//! │                                              │
//! │  Value: [________________] [Authorize]       │
//! └──────────────────────────────────────────────┘
//! ```
//!
//! # Security Requirements
//!
//! Once a security scheme is defined, it can be applied to operations:
//!
//! ```rust,ignore
//! #[utoipa::path(
//!     get,
//!     security(("bearerAuth" = []))
//! )]
//! ```
//!
//! This adds a lock icon in Swagger UI indicating the endpoint requires
//! authentication.

use proc_macro2::TokenStream;
use quote::quote;

/// Generates security scheme code for the `Modify` implementation.
///
/// This function produces code that registers a security scheme in the
/// OpenAPI components section. The scheme defines how the API authenticates
/// requests and is displayed in Swagger UI's "Authorize" dialog.
///
/// # Arguments
///
/// * `security` - Optional security type string: `"bearer"`, `"cookie"`, or
///   `"api_key"`
///
/// # Returns
///
/// A `TokenStream` containing code to add the security scheme to components.
/// Returns empty stream if security is `None` or unrecognized.
///
/// # Security Type Mapping
///
/// | Input | Scheme Name | Type |
/// |-------|-------------|------|
/// | `"bearer"` | `bearerAuth` | HTTP Bearer with JWT format |
/// | `"cookie"` | `cookieAuth` | API Key in cookie named "token" |
/// | `"api_key"` | `apiKey` | API Key in "X-API-Key" header |
///
/// # Usage
///
/// Called within `generate_modifier()` to add security schemes:
///
/// ```rust,ignore
/// let security_code = generate_security_code(api_config.security.as_deref());
///
/// quote! {
///     fn modify(&self, openapi: &mut OpenApi) {
///         #security_code  // Adds scheme to components
///     }
/// }
/// ```
pub fn generate_security_code(security: Option<&str>) -> TokenStream {
    let Some(security) = security else {
        return TokenStream::new();
    };

    let (scheme_name, scheme_impl) = match security {
        "cookie" => (
            "cookieAuth",
            quote! {
                security::SecurityScheme::ApiKey(
                    security::ApiKey::Cookie(
                        security::ApiKeyValue::with_description(
                            "token",
                            "JWT token stored in HTTP-only cookie"
                        )
                    )
                )
            }
        ),
        "bearer" => (
            "bearerAuth",
            quote! {
                security::SecurityScheme::Http(
                    security::HttpBuilder::new()
                        .scheme(security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT token in Authorization header"))
                        .build()
                )
            }
        ),
        "api_key" => (
            "apiKey",
            quote! {
                security::SecurityScheme::ApiKey(
                    security::ApiKey::Header(
                        security::ApiKeyValue::with_description(
                            "X-API-Key",
                            "API key for service-to-service authentication"
                        )
                    )
                )
            }
        ),
        _ => return TokenStream::new()
    };

    quote! {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(#scheme_name, #scheme_impl);
        }
    }
}

/// Returns the OpenAPI security scheme name for a given security type.
///
/// This function maps user-facing security type names to their corresponding
/// OpenAPI security scheme identifiers. The scheme name is used both when
/// defining the security scheme and when applying it to operations.
///
/// # Arguments
///
/// * `security` - The security type: `"bearer"`, `"cookie"`, or `"api_key"`
///
/// # Returns
///
/// The canonical OpenAPI scheme name used throughout the specification.
///
/// # Mapping
///
/// | Input | Output | Description |
/// |-------|--------|-------------|
/// | `"bearer"` | `"bearerAuth"` | JWT in Authorization header |
/// | `"cookie"` | `"cookieAuth"` | JWT in HTTP-only cookie |
/// | `"api_key"` | `"apiKey"` | Key in X-API-Key header |
/// | other | `"cookieAuth"` | Default fallback |
///
/// # Usage
///
/// The scheme name is used in two places:
///
/// 1. **Defining the scheme** (in components/securitySchemes): ```rust,ignore
///    components.add_security_scheme("bearerAuth", scheme); ```
///
/// 2. **Applying to operations** (in path operations): ```rust,ignore
///    security::SecurityRequirement::new::<_, _, &str>("bearerAuth", []) ```
///
/// # Consistency
///
/// The same scheme name must be used in both places. This function ensures
/// consistency by providing a single source of truth for the mapping.
pub fn security_scheme_name(security: &str) -> &'static str {
    match security {
        "cookie" => "cookieAuth",
        "bearer" => "bearerAuth",
        "api_key" => "apiKey",
        _ => "cookieAuth"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_code_none() {
        let code = generate_security_code(None);
        assert!(code.is_empty());
    }

    #[test]
    fn security_code_cookie() {
        let code = generate_security_code(Some("cookie"));
        let code_str = code.to_string();
        assert!(code_str.contains("cookieAuth"));
        assert!(code_str.contains("Cookie"));
        assert!(code_str.contains("token"));
    }

    #[test]
    fn security_code_bearer() {
        let code = generate_security_code(Some("bearer"));
        let code_str = code.to_string();
        assert!(code_str.contains("bearerAuth"));
        assert!(code_str.contains("Bearer"));
        assert!(code_str.contains("JWT"));
    }

    #[test]
    fn security_code_api_key() {
        let code = generate_security_code(Some("api_key"));
        let code_str = code.to_string();
        assert!(code_str.contains("apiKey"));
        assert!(code_str.contains("Header"));
        assert!(code_str.contains("X-API-Key"));
    }

    #[test]
    fn security_code_unknown_returns_empty() {
        let code = generate_security_code(Some("unknown"));
        assert!(code.is_empty());
    }

    #[test]
    fn scheme_name_cookie() {
        assert_eq!(security_scheme_name("cookie"), "cookieAuth");
    }

    #[test]
    fn scheme_name_bearer() {
        assert_eq!(security_scheme_name("bearer"), "bearerAuth");
    }

    #[test]
    fn scheme_name_api_key() {
        assert_eq!(security_scheme_name("api_key"), "apiKey");
    }

    #[test]
    fn scheme_name_unknown_defaults_to_cookie() {
        assert_eq!(security_scheme_name("unknown"), "cookieAuth");
        assert_eq!(security_scheme_name(""), "cookieAuth");
        assert_eq!(security_scheme_name("jwt"), "cookieAuth");
    }
}
