// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! API configuration parsing from `#[entity(api(...))]` attributes.
//!
//! This module provides the parser that extracts API configuration from
//! the `api(...)` nested attribute within `#[entity(...)]`. It validates
//! syntax, handles all configuration options, and produces an `ApiConfig`.
//!
//! # Parsing Flow
//!
//! ```text
//! Input Attribute                      Parser                    Output
//!
//! #[entity(                        parse_api_config()
//!   api(                                 │
//!     tag = "Users",      ──────────────►├── tag = Some("Users")
//!     security = "bearer", ─────────────►├── security = Some("bearer")
//!     handlers(create, get) ────────────►├── handlers.create = true
//!   )                                    │   handlers.get = true
//! )]                                     ▼
//!                                   ApiConfig { ... }
//! ```
//!
//! # Supported Syntax
//!
//! The parser handles multiple attribute forms:
//!
//! ## String Values
//!
//! ```rust,ignore
//! api(tag = "Users")           // Simple string
//! api(path_prefix = "/api/v1") // Path string
//! ```
//!
//! ## Boolean Values
//!
//! ```rust,ignore
//! api(handlers = true)   // Explicit boolean
//! api(handlers = false)  // Disable handlers
//! ```
//!
//! ## Flags
//!
//! ```rust,ignore
//! api(handlers)   // Equivalent to handlers = true
//! ```
//!
//! ## Lists
//!
//! ```rust,ignore
//! api(public = [Login, Register])     // Bracketed list
//! api(handlers(create, get, list))    // Parenthesized list
//! ```
//!
//! # Error Handling
//!
//! The parser provides clear error messages for invalid syntax:
//!
//! ```text
//! error: api attribute requires parameters: api(tag = "...")
//!   --> src/lib.rs:5:3
//!    |
//!  5 | #[entity(api)]
//!    |          ^^^
//!
//! error: unknown api option 'unknown_option', expected: tag, ...
//!   --> src/lib.rs:5:7
//!    |
//!  5 | #[entity(api(unknown_option = "value"))]
//!    |              ^^^^^^^^^^^^^^
//! ```
//!
//! # Option Reference
//!
//! | Option | Syntax | Type |
//! |--------|--------|------|
//! | `tag` | `tag = "..."` | String |
//! | `tag_description` | `tag_description = "..."` | String |
//! | `path_prefix` | `path_prefix = "..."` | String |
//! | `security` | `security = "..."` | String |
//! | `public` | `public = [A, B]` | List of Idents |
//! | `version` | `version = "..."` | String |
//! | `deprecated_in` | `deprecated_in = "..."` | String |
//! | `handlers` | `handlers` / `handlers(...)` / `handlers = bool` | Flag/List/Bool |
//! | `title` | `title = "..."` | String |
//! | `description` | `description = "..."` | String |
//! | `api_version` | `api_version = "..."` | String |
//! | `license` | `license = "..."` | String |
//! | `license_url` | `license_url = "..."` | String |
//! | `contact_name` | `contact_name = "..."` | String |
//! | `contact_email` | `contact_email = "..."` | String |
//! | `contact_url` | `contact_url = "..."` | String |

use syn::Ident;

use super::config::{ApiConfig, HandlerConfig};

/// Parses the `#[entity(api(...))]` attribute into an [`ApiConfig`].
///
/// This function extracts all API configuration options from the nested
/// `api(...)` attribute. It validates the syntax and returns helpful
/// error messages for invalid input.
///
/// # Arguments
///
/// * `meta` - The `syn::Meta` representing the `api(...)` attribute
///
/// # Returns
///
/// - `Ok(ApiConfig)` - Successfully parsed configuration
/// - `Err(syn::Error)` - Syntax error with span information
///
/// # Parsing Process
///
/// ```text
/// syn::Meta::List("api(...)")
///        │
///        ▼
/// parse_nested_meta(|nested| {
///     match nested.path {
///         "tag" → config.tag = Some(value)
///         "handlers" → parse handlers syntax
///         ...
///     }
/// })
///        │
///        ▼
///    ApiConfig
/// ```
///
/// # Handler Parsing
///
/// The `handlers` option has special parsing logic:
///
/// | Syntax | Interpretation |
/// |--------|----------------|
/// | `handlers` | Enable all handlers |
/// | `handlers = true` | Enable all handlers |
/// | `handlers = false` | Disable all handlers |
/// | `handlers(create, get)` | Enable specific handlers |
///
/// # Error Cases
///
/// | Input | Error |
/// |-------|-------|
/// | `api` | "api attribute requires parameters" |
/// | `api = "value"` | "api attribute must use parentheses" |
/// | `api(unknown = "x")` | "unknown api option 'unknown'" |
/// | `api(handlers(invalid))` | "unknown handler 'invalid'" |
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
                                        "unknown handler '{}', expected: create, get, update, \
                                         delete, list",
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
                         api_version, license, license_url, contact_name, contact_email, \
                         contact_url",
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

    #[test]
    fn parse_path_only_fails() {
        let attr: syn::Attribute = syn::parse_quote!(#[api]);
        let result = parse_api_config(&attr.meta);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires parameters")
        );
    }

    #[test]
    fn parse_name_value_fails() {
        let attr: syn::Attribute = syn::parse_quote!(#[api = "value"]);
        let result = parse_api_config(&attr.meta);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parentheses"));
    }

    #[test]
    fn parse_tag() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(tag = "Users")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.tag, Some("Users".to_string()));
    }

    #[test]
    fn parse_tag_description() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(tag_description = "User management")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.tag_description, Some("User management".to_string()));
    }

    #[test]
    fn parse_path_prefix() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(path_prefix = "/api/v1")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.path_prefix, Some("/api/v1".to_string()));
    }

    #[test]
    fn parse_security() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(security = "bearer")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.security, Some("bearer".to_string()));
    }

    #[test]
    fn parse_version() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(version = "v2")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.version, Some("v2".to_string()));
    }

    #[test]
    fn parse_deprecated_in() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(deprecated_in = "2.0")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.deprecated_in, Some("2.0".to_string()));
    }

    #[test]
    fn parse_handlers_flag() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(handlers)]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert!(config.handlers.any());
        assert!(config.handlers.create);
        assert!(config.handlers.get);
        assert!(config.handlers.update);
        assert!(config.handlers.delete);
        assert!(config.handlers.list);
    }

    #[test]
    fn parse_handlers_true() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(handlers = true)]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert!(config.handlers.any());
    }

    #[test]
    fn parse_handlers_false() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(handlers = false)]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert!(!config.handlers.any());
    }

    #[test]
    fn parse_handlers_selective() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(handlers(create, get))]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert!(config.handlers.create);
        assert!(config.handlers.get);
        assert!(!config.handlers.update);
        assert!(!config.handlers.delete);
        assert!(!config.handlers.list);
    }

    #[test]
    fn parse_handlers_all_selective() {
        let attr: syn::Attribute =
            syn::parse_quote!(#[api(handlers(create, get, update, delete, list))]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert!(config.handlers.create);
        assert!(config.handlers.get);
        assert!(config.handlers.update);
        assert!(config.handlers.delete);
        assert!(config.handlers.list);
    }

    #[test]
    fn parse_handlers_invalid() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(handlers(invalid))]);
        let result = parse_api_config(&attr.meta);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown handler"));
    }

    #[test]
    fn parse_title() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(title = "My API")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.title, Some("My API".to_string()));
    }

    #[test]
    fn parse_description() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(description = "API description")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.description, Some("API description".to_string()));
    }

    #[test]
    fn parse_api_version() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(api_version = "1.0.0")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.api_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn parse_license() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(license = "MIT")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.license, Some("MIT".to_string()));
    }

    #[test]
    fn parse_license_url() {
        let attr: syn::Attribute =
            syn::parse_quote!(#[api(license_url = "https://mit.edu/license")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(
            config.license_url,
            Some("https://mit.edu/license".to_string())
        );
    }

    #[test]
    fn parse_contact_name() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(contact_name = "John Doe")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.contact_name, Some("John Doe".to_string()));
    }

    #[test]
    fn parse_contact_email() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(contact_email = "john@example.com")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.contact_email, Some("john@example.com".to_string()));
    }

    #[test]
    fn parse_contact_url() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(contact_url = "https://example.com")]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.contact_url, Some("https://example.com".to_string()));
    }

    #[test]
    fn parse_unknown_option() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(unknown_option = "value")]);
        let result = parse_api_config(&attr.meta);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unknown api option")
        );
    }

    #[test]
    fn parse_multiple_options() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(
            tag = "Users",
            path_prefix = "/api/v1",
            security = "bearer",
            handlers(create, get)
        )]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.tag, Some("Users".to_string()));
        assert_eq!(config.path_prefix, Some("/api/v1".to_string()));
        assert_eq!(config.security, Some("bearer".to_string()));
        assert!(config.handlers.create);
        assert!(config.handlers.get);
        assert!(!config.handlers.update);
    }

    #[test]
    fn parse_public_commands() {
        let attr: syn::Attribute = syn::parse_quote!(#[api(public = [Login, Register])]);
        let config = parse_api_config(&attr.meta).unwrap();
        assert_eq!(config.public_commands.len(), 2);
        assert!(config.public_commands.iter().any(|i| i == "Login"));
        assert!(config.public_commands.iter().any(|i| i == "Register"));
    }
}
