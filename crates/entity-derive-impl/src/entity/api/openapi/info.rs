// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI Info section generation.
//!
//! This module generates code to configure the OpenAPI specification's info
//! object, which provides metadata about the API. The info section is required
//! by OpenAPI 3.0+ and appears at the top level of the specification.
//!
//! # OpenAPI Info Object
//!
//! According to the OpenAPI 3.0 specification, the info object contains:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     OpenAPI Info Object                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  Required Fields                                                │
//! │  ├─► title:   API name displayed in Swagger UI                  │
//! │  └─► version: API version string (e.g., "1.0.0")                │
//! │                                                                 │
//! │  Optional Fields                                                │
//! │  ├─► description: Detailed API description (markdown)           │
//! │  ├─► license:     License information                           │
//! │  │   ├─► name: License name (e.g., "MIT")                       │
//! │  │   └─► url:  License URL                                      │
//! │  └─► contact:     API maintainer information                    │
//! │      ├─► name:  Contact person/organization                     │
//! │      ├─► email: Support email                                   │
//! │      └─► url:   Support website                                 │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Configuration Sources
//!
//! Info fields are populated from the `#[entity(api(...))]` attribute:
//!
//! | Attribute | Info Field | Default |
//! |-----------|------------|---------|
//! | `title` | `info.title` | None |
//! | `description` | `info.description` | Entity doc comment |
//! | `api_version` | `info.version` | None |
//! | `license` | `info.license.name` | None |
//! | `license_url` | `info.license.url` | None |
//! | `contact_name` | `info.contact.name` | None |
//! | `contact_email` | `info.contact.email` | None |
//! | `contact_url` | `info.contact.url` | None |
//!
//! # Generated Code Example
//!
//! For an entity with full info configuration:
//!
//! ```rust,ignore
//! #[entity(
//!     table = "users",
//!     api(
//!         title = "User API",
//!         description = "Manage user accounts",
//!         api_version = "2.0.0",
//!         license = "MIT",
//!         license_url = "https://opensource.org/licenses/MIT",
//!         contact_name = "API Team",
//!         contact_email = "api@example.com",
//!         handlers
//!     )
//! )]
//! pub struct User { ... }
//! ```
//!
//! Generates:
//!
//! ```rust,ignore
//! openapi.info.title = "User API".to_string();
//! openapi.info.description = Some("Manage user accounts".to_string());
//! openapi.info.version = "2.0.0".to_string();
//! openapi.info.license = Some(
//!     info::LicenseBuilder::new()
//!         .name("MIT")
//!         .url(Some("https://opensource.org/licenses/MIT"))
//!         .build()
//! );
//! openapi.info.contact = Some(
//!     info::ContactBuilder::new()
//!         .name(Some("API Team"))
//!         .email(Some("api@example.com"))
//!         .build()
//! );
//! ```
//!
//! # Deprecation Notice
//!
//! When `#[entity(api(deprecated))]` or `deprecated_in = "x.x.x"` is set,
//! the description is prefixed with a deprecation warning:
//!
//! ```text
//! **DEPRECATED**: Deprecated since 1.5.0
//!
//! Original description here...
//! ```
//!
//! # Swagger UI Rendering
//!
//! The info section appears prominently in Swagger UI:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  User API                                    v2.0.0      │
//! │  ────────────────────────────────────────────────────────│
//! │  Manage user accounts                                    │
//! │                                                          │
//! │  License: MIT                                            │
//! │  Contact: API Team <api@example.com>                     │
//! └──────────────────────────────────────────────────────────┘
//! ```

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityDef;

/// Generates code to configure the OpenAPI info section.
///
/// This function produces a `TokenStream` that sets various properties on
/// `openapi.info` within the `Modify::modify()` implementation. Only configured
/// fields are set; unconfigured fields retain their default values.
///
/// # Arguments
///
/// * `entity` - The parsed entity definition containing API configuration
///
/// # Returns
///
/// A `TokenStream` containing assignment statements for the info object.
/// May be empty if no info fields are configured.
///
/// # Field Generation
///
/// ```text
/// ApiConfig
///     │
///     ├─► title ─────────────► openapi.info.title = ...
///     ├─► description ───────► openapi.info.description = Some(...)
///     │   └─► or entity doc
///     ├─► api_version ───────► openapi.info.version = ...
///     ├─► license ───────────► openapi.info.license = Some(...)
///     │   └─► license_url ───► .url(Some(...))
///     ├─► contact_* ─────────► openapi.info.contact = Some(...)
///     │   ├─► contact_name ──► .name(Some(...))
///     │   ├─► contact_email ─► .email(Some(...))
///     │   └─► contact_url ───► .url(Some(...))
///     └─► deprecated ────────► Prepend warning to description
/// ```
///
/// # Builder Pattern
///
/// License and contact use utoipa's builder pattern:
///
/// ```rust,ignore
/// info::LicenseBuilder::new()
///     .name("MIT")
///     .url(Some("https://..."))
///     .build()
/// ```
///
/// This ensures type safety and proper optional field handling.
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
