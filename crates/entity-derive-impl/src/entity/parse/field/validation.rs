// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Validation attribute parsing.
//!
//! Extracts `#[validate(...)]` attributes from fields for:
//! - Passing through to generated DTOs
//! - Converting to OpenAPI schema constraints
//!
//! # Supported Validators
//!
//! | Validator | OpenAPI Constraint |
//! |-----------|-------------------|
//! | `length(min = N)` | `minLength: N` |
//! | `length(max = N)` | `maxLength: N` |
//! | `range(min = N)` | `minimum: N` |
//! | `range(max = N)` | `maximum: N` |
//! | `email` | `format: email` |
//! | `url` | `format: uri` |
//! | `regex = "..."` | `pattern: ...` |
//!
//! # Example
//!
//! ```rust,ignore
//! #[validate(length(min = 1, max = 255))]
//! #[validate(email)]
//! pub email: String,
//!
//! // Generates in OpenAPI schema:
//! // email:
//! //   type: string
//! //   minLength: 1
//! //   maxLength: 255
//! //   format: email
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::Attribute;

/// Parsed validation configuration from `#[validate(...)]` attributes.
#[derive(Debug, Clone, Default)]
pub struct ValidationConfig {
    /// Minimum string length.
    pub min_length: Option<usize>,

    /// Maximum string length.
    pub max_length: Option<usize>,

    /// Minimum numeric value.
    pub minimum: Option<i64>,

    /// Maximum numeric value.
    pub maximum: Option<i64>,

    /// Email format validation.
    pub email: bool,

    /// URL format validation.
    pub url: bool,

    /// Regex pattern.
    pub pattern: Option<String>,

    /// Raw validate attributes to pass through.
    pub raw_attrs: Vec<TokenStream>
}

impl ValidationConfig {
    /// Check if any validation is configured.
    #[must_use]
    #[allow(dead_code)] // Will be used when generating schema constraints
    pub fn has_validation(&self) -> bool {
        self.min_length.is_some()
            || self.max_length.is_some()
            || self.minimum.is_some()
            || self.maximum.is_some()
            || self.email
            || self.url
            || self.pattern.is_some()
    }

    /// Generate OpenAPI schema attributes for utoipa.
    ///
    /// Returns TokenStream with schema constraints like `min_length = N`.
    #[must_use]
    #[allow(dead_code)] // Will be used when generating schema constraints
    pub fn to_schema_attrs(&self) -> TokenStream {
        let mut attrs = Vec::new();

        if let Some(min) = self.min_length {
            attrs.push(quote! { min_length = #min });
        }
        if let Some(max) = self.max_length {
            attrs.push(quote! { max_length = #max });
        }
        if let Some(min) = self.minimum {
            attrs.push(quote! { minimum = #min });
        }
        if let Some(max) = self.maximum {
            attrs.push(quote! { maximum = #max });
        }
        if self.email {
            attrs.push(quote! { format = "email" });
        }
        if self.url {
            attrs.push(quote! { format = "uri" });
        }
        if let Some(ref pattern) = self.pattern {
            attrs.push(quote! { pattern = #pattern });
        }

        if attrs.is_empty() {
            TokenStream::new()
        } else {
            quote! { #(, #attrs)* }
        }
    }
}

/// Parse validation attributes from a field.
///
/// Extracts all `#[validate(...)]` attributes and parses their content.
pub fn parse_validation_attrs(attrs: &[Attribute]) -> ValidationConfig {
    let mut config = ValidationConfig::default();

    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        // Store raw attribute for passthrough
        config.raw_attrs.push(quote! { #attr });

        // Parse the attribute content
        let _ = attr.parse_nested_meta(|meta| {
            let path_str = meta.path.get_ident().map(|i| i.to_string());

            match path_str.as_deref() {
                Some("length") => {
                    meta.parse_nested_meta(|nested| {
                        let nested_path = nested.path.get_ident().map(|i| i.to_string());
                        match nested_path.as_deref() {
                            Some("min") => {
                                let value: syn::LitInt = nested.value()?.parse()?;
                                config.min_length = Some(value.base10_parse()?);
                            }
                            Some("max") => {
                                let value: syn::LitInt = nested.value()?.parse()?;
                                config.max_length = Some(value.base10_parse()?);
                            }
                            _ => {}
                        }
                        Ok(())
                    })?;
                }
                Some("range") => {
                    meta.parse_nested_meta(|nested| {
                        let nested_path = nested.path.get_ident().map(|i| i.to_string());
                        match nested_path.as_deref() {
                            Some("min") => {
                                let value: syn::LitInt = nested.value()?.parse()?;
                                config.minimum = Some(value.base10_parse()?);
                            }
                            Some("max") => {
                                let value: syn::LitInt = nested.value()?.parse()?;
                                config.maximum = Some(value.base10_parse()?);
                            }
                            _ => {}
                        }
                        Ok(())
                    })?;
                }
                Some("email") => {
                    config.email = true;
                }
                Some("url") => {
                    config.url = true;
                }
                Some("regex") => {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    config.pattern = Some(value.value());
                }
                _ => {}
            }

            Ok(())
        });
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_attrs(input: &str) -> Vec<Attribute> {
        let item: syn::ItemStruct = syn::parse_str(input).unwrap();
        item.fields
            .iter()
            .next()
            .map(|f| f.attrs.clone())
            .unwrap_or_default()
    }

    #[test]
    fn parse_length_min_max() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[validate(length(min = 1, max = 255))]
                name: String,
            }
        "#
        );
        let config = parse_validation_attrs(&attrs);
        assert_eq!(config.min_length, Some(1));
        assert_eq!(config.max_length, Some(255));
    }

    #[test]
    fn parse_email() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[validate(email)]
                email: String,
            }
        "#
        );
        let config = parse_validation_attrs(&attrs);
        assert!(config.email);
    }

    #[test]
    fn parse_url() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[validate(url)]
                website: String,
            }
        "#
        );
        let config = parse_validation_attrs(&attrs);
        assert!(config.url);
    }

    #[test]
    fn parse_range() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[validate(range(min = 0, max = 100))]
                score: i32,
            }
        "#
        );
        let config = parse_validation_attrs(&attrs);
        assert_eq!(config.minimum, Some(0));
        assert_eq!(config.maximum, Some(100));
    }

    #[test]
    fn parse_multiple_validators() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[validate(length(min = 5))]
                #[validate(email)]
                email: String,
            }
        "#
        );
        let config = parse_validation_attrs(&attrs);
        assert_eq!(config.min_length, Some(5));
        assert!(config.email);
    }

    #[test]
    fn no_validation() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[field(create)]
                name: String,
            }
        "#
        );
        let config = parse_validation_attrs(&attrs);
        assert!(!config.has_validation());
    }

    #[test]
    fn has_validation_true() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[validate(email)]
                email: String,
            }
        "#
        );
        let config = parse_validation_attrs(&attrs);
        assert!(config.has_validation());
    }

    #[test]
    fn schema_attrs_generation() {
        let config = ValidationConfig {
            min_length: Some(1),
            max_length: Some(100),
            email: true,
            ..Default::default()
        };

        let attrs = config.to_schema_attrs();
        let attrs_str = attrs.to_string();

        assert!(attrs_str.contains("min_length"));
        assert!(attrs_str.contains("max_length"));
        assert!(attrs_str.contains("email"));
    }
}
