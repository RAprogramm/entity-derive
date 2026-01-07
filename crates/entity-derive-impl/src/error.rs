// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! EntityError derive macro implementation.
//!
//! Generates OpenAPI error response documentation from enum variants.
//!
//! # Example
//!
//! ```rust,ignore
//! #[derive(Debug, Error, ToSchema, EntityError)]
//! pub enum UserError {
//!     /// User with this email already exists
//!     #[error("Email already exists")]
//!     #[status(409)]
//!     EmailExists,
//!
//!     /// User not found by ID
//!     #[error("User not found")]
//!     #[status(404)]
//!     NotFound,
//! }
//! ```
//!
//! Generates `UserErrorResponses` that can be used in handlers.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Attribute, DeriveInput, parse_macro_input};

use crate::utils::docs::extract_doc_summary;

/// Main entry point for the EntityError derive macro.
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into()
    }
}

/// Generate the error responses code.
fn generate(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let vis = &input.vis;

    // Ensure it's an enum
    let variants = match &input.data {
        syn::Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "EntityError can only be derived for enums"
            ));
        }
    };

    // Parse error variants
    let error_variants: Vec<ErrorVariant> = variants
        .iter()
        .filter_map(|v| parse_error_variant(v).ok())
        .collect();

    if error_variants.is_empty() {
        return Ok(TokenStream2::new());
    }

    // Generate responses struct name
    let responses_struct = format_ident!("{}Responses", name);

    // Generate status codes array
    let status_codes: Vec<u16> = error_variants.iter().map(|v| v.status).collect();

    // Generate descriptions array
    let descriptions: Vec<&String> = error_variants.iter().map(|v| &v.description).collect();

    let doc = format!(
        "OpenAPI error responses for `{}`.\\n\\n\
         Use with `#[utoipa::path(responses(...))]`.",
        name
    );

    Ok(quote! {
        #[doc = #doc]
        #vis struct #responses_struct;

        impl #responses_struct {
            /// Get all error status codes.
            #[must_use]
            pub const fn status_codes() -> &'static [u16] {
                &[#(#status_codes),*]
            }

            /// Get all error descriptions.
            #[must_use]
            pub fn descriptions() -> &'static [&'static str] {
                &[#(#descriptions),*]
            }

            /// Generate utoipa response entries.
            ///
            /// Use in `#[utoipa::path(responses(...))]`.
            #[must_use]
            pub fn utoipa_responses() -> Vec<(u16, &'static str)> {
                vec![
                    #((#status_codes, #descriptions)),*
                ]
            }
        }
    })
}

/// Parsed error variant.
struct ErrorVariant {
    /// HTTP status code from `#[status(code)]`.
    status:      u16,
    /// Description from doc comment.
    description: String
}

/// Parse a single enum variant for error info.
fn parse_error_variant(variant: &syn::Variant) -> syn::Result<ErrorVariant> {
    let status = parse_status_attr(&variant.attrs)?;
    let description =
        extract_doc_summary(&variant.attrs).unwrap_or_else(|| format!("{} error", variant.ident));

    Ok(ErrorVariant {
        status,
        description
    })
}

/// Parse `#[status(code)]` attribute.
fn parse_status_attr(attrs: &[Attribute]) -> syn::Result<u16> {
    for attr in attrs {
        if attr.path().is_ident("status") {
            let status: syn::LitInt = attr.parse_args()?;
            return status.base10_parse();
        }
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "Missing #[status(code)] attribute"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_code() {
        let input: DeriveInput = syn::parse_quote! {
            enum UserError {
                /// User not found
                #[status(404)]
                NotFound,
            }
        };

        if let syn::Data::Enum(data) = &input.data {
            let variant = &data.variants[0];
            let status = parse_status_attr(&variant.attrs).unwrap();
            assert_eq!(status, 404);
        }
    }

    #[test]
    fn parse_error_variant_full() {
        let input: DeriveInput = syn::parse_quote! {
            enum UserError {
                /// User with this email already exists
                #[status(409)]
                EmailExists,
            }
        };

        if let syn::Data::Enum(data) = &input.data {
            let variant = &data.variants[0];
            let parsed = parse_error_variant(variant).unwrap();
            assert_eq!(parsed.status, 409);
            assert_eq!(parsed.description, "User with this email already exists");
        }
    }

    #[test]
    fn parse_missing_status_fails() {
        let input: DeriveInput = syn::parse_quote! {
            enum UserError {
                /// Some error
                NoStatus,
            }
        };

        if let syn::Data::Enum(data) = &input.data {
            let variant = &data.variants[0];
            let result = parse_error_variant(variant);
            assert!(result.is_err());
        }
    }

    #[test]
    fn generate_for_non_enum_fails() {
        let input: DeriveInput = syn::parse_quote! {
            struct NotAnEnum {
                field: String,
            }
        };

        let result = generate(&input);
        assert!(result.is_err());
    }

    #[test]
    fn generate_empty_variants_returns_empty() {
        let input: DeriveInput = syn::parse_quote! {
            enum EmptyError {
                NoStatus,
            }
        };

        let result = generate(&input);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn generate_multiple_variants() {
        let input: DeriveInput = syn::parse_quote! {
            enum UserError {
                /// User not found
                #[status(404)]
                NotFound,
                /// Already exists
                #[status(409)]
                AlreadyExists,
                /// Internal error
                #[status(500)]
                Internal,
            }
        };

        let result = generate(&input);
        assert!(result.is_ok());
        let output = result.unwrap().to_string();
        assert!(output.contains("UserErrorResponses"));
        assert!(output.contains("status_codes"));
        assert!(output.contains("descriptions"));
        assert!(output.contains("utoipa_responses"));
        assert!(output.contains("404"));
        assert!(output.contains("409"));
        assert!(output.contains("500"));
    }

    #[test]
    fn parse_variant_without_doc_uses_default() {
        let input: DeriveInput = syn::parse_quote! {
            enum Error {
                #[status(400)]
                BadRequest,
            }
        };

        if let syn::Data::Enum(data) = &input.data {
            let variant = &data.variants[0];
            let parsed = parse_error_variant(variant).unwrap();
            assert_eq!(parsed.status, 400);
            assert!(parsed.description.contains("BadRequest"));
        }
    }

    #[test]
    fn generate_public_visibility() {
        let input: DeriveInput = syn::parse_quote! {
            pub enum ApiError {
                /// Not found
                #[status(404)]
                NotFound,
            }
        };

        let result = generate(&input);
        assert!(result.is_ok());
        let output = result.unwrap().to_string();
        assert!(output.contains("pub struct ApiErrorResponses"));
    }

    #[test]
    fn generate_private_visibility() {
        let input: DeriveInput = syn::parse_quote! {
            enum PrivateError {
                /// Error
                #[status(500)]
                Internal,
            }
        };

        let result = generate(&input);
        assert!(result.is_ok());
        let output = result.unwrap().to_string();
        assert!(output.contains("struct PrivateErrorResponses"));
        assert!(!output.contains("pub struct PrivateErrorResponses"));
    }

    #[test]
    fn status_code_parsing_various_codes() {
        let codes = [200_u16, 201, 400, 401, 403, 404, 409, 422, 500, 502, 503];
        for code in codes {
            let code_str = code.to_string();
            let input: DeriveInput = syn::parse_quote! {
                enum Error {
                    /// Test
                    #[status(#code)]
                    Test,
                }
            };

            if let syn::Data::Enum(data) = &input.data {
                let variant = &data.variants[0];
                let result = parse_status_attr(&variant.attrs);
                assert!(
                    result.is_ok(),
                    "Should parse status code {}",
                    code_str
                );
            }
        }
    }
}
