// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! API configuration parsing.

use syn::Ident;

use super::config::{ApiConfig, HandlerConfig};

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
