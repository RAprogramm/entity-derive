// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI info section generation.
//!
//! Generates code to configure the OpenAPI info section including title,
//! description, version, license, contact information, and deprecation status.

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityDef;

/// Generate code to configure OpenAPI info section.
///
/// Sets title, description, version, license, and contact information.
pub fn generate_info_code(entity: &EntityDef) -> TokenStream {
    let api_config = entity.api_config();

    let title_code = if let Some(ref title) = api_config.title {
        quote! { openapi.info.title = #title.to_string(); }
    } else {
        TokenStream::new()
    };

    let description_code = if let Some(ref description) = api_config.description {
        quote! { openapi.info.description = Some(#description.to_string()); }
    } else if let Some(doc) = entity.doc() {
        quote! { openapi.info.description = Some(#doc.to_string()); }
    } else {
        TokenStream::new()
    };

    let version_code = if let Some(ref version) = api_config.api_version {
        quote! { openapi.info.version = #version.to_string(); }
    } else {
        TokenStream::new()
    };

    let license_code = match (&api_config.license, &api_config.license_url) {
        (Some(name), Some(url)) => {
            quote! {
                openapi.info.license = Some(
                    info::LicenseBuilder::new()
                        .name(#name)
                        .url(Some(#url))
                        .build()
                );
            }
        }
        (Some(name), None) => {
            quote! {
                openapi.info.license = Some(
                    info::LicenseBuilder::new()
                        .name(#name)
                        .build()
                );
            }
        }
        _ => TokenStream::new()
    };

    let has_contact = api_config.contact_name.is_some()
        || api_config.contact_email.is_some()
        || api_config.contact_url.is_some();

    let contact_code = if has_contact {
        let name = api_config.contact_name.as_deref().unwrap_or("");
        let email = api_config.contact_email.as_deref();
        let url = api_config.contact_url.as_deref();

        let email_setter = if let Some(e) = email {
            quote! { .email(Some(#e)) }
        } else {
            TokenStream::new()
        };

        let url_setter = if let Some(u) = url {
            quote! { .url(Some(#u)) }
        } else {
            TokenStream::new()
        };

        quote! {
            openapi.info.contact = Some(
                info::ContactBuilder::new()
                    .name(Some(#name))
                    #email_setter
                    #url_setter
                    .build()
            );
        }
    } else {
        TokenStream::new()
    };

    let deprecated_code = if api_config.is_deprecated() {
        let version = api_config.deprecated_in.as_deref().unwrap_or("unknown");
        let msg = format!("Deprecated since {}", version);
        quote! {
            if let Some(ref desc) = openapi.info.description {
                openapi.info.description = Some(format!("**DEPRECATED**: {}\n\n{}", #msg, desc));
            } else {
                openapi.info.description = Some(format!("**DEPRECATED**: {}", #msg));
            }
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #title_code
        #description_code
        #version_code
        #license_code
        #contact_code
        #deprecated_code
    }
}
