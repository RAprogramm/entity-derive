// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI security scheme generation.
//!
//! Generates security scheme code for cookie, bearer, and API key
//! authentication.

use proc_macro2::TokenStream;
use quote::quote;

/// Generate security scheme code for the Modify implementation.
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

/// Get the security scheme name for a given security type.
pub fn security_scheme_name(security: &str) -> &'static str {
    match security {
        "cookie" => "cookieAuth",
        "bearer" => "bearerAuth",
        "api_key" => "apiKey",
        _ => "cookieAuth"
    }
}
