// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

#![allow(dead_code)]

//! API configuration parsing for OpenAPI/utoipa integration.
//!
//! This module handles parsing of `#[entity(api(...))]` attributes that control
//! automatic HTTP handler generation with OpenAPI documentation. The API
//! configuration determines what handlers are generated, how they're secured,
//! and how they appear in Swagger UI.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    API Configuration Parsing                        │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Source                 Parsing                    Output           │
//! │                                                                     │
//! │  #[entity(             parse_api_config()         ApiConfig         │
//! │    api(                      │                        │             │
//! │      tag = "Users",          │                        ├── tag       │
//! │      security = "bearer",    │                        ├── security  │
//! │      handlers(create, get)   │                        ├── handlers  │
//! │    )                         │                        └── ...       │
//! │  )]                          ▼                                      │
//! │                         HandlerConfig                               │
//! │                             │                                       │
//! │                             ├── create: true                        │
//! │                             ├── get: true                           │
//! │                             └── update/delete/list: false           │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Configuration Options
//!
//! The `api(...)` attribute supports the following options:
//!
//! ## Core Options
//!
//! | Option | Type | Required | Description |
//! |--------|------|----------|-------------|
//! | `tag` | string | Yes | OpenAPI tag for endpoint grouping |
//! | `tag_description` | string | No | Tag description for docs |
//! | `handlers` | flag/list | No | CRUD handlers to generate |
//!
//! ## URL Configuration
//!
//! | Option | Type | Example | Result |
//! |--------|------|---------|--------|
//! | `path_prefix` | string | `"/api"` | `/api/users` |
//! | `version` | string | `"v1"` | `/api/v1/users` |
//!
//! ## Security Configuration
//!
//! | Option | Type | Values | Description |
//! |--------|------|--------|-------------|
//! | `security` | string | `"bearer"`, `"cookie"`, `"api_key"` | Default auth |
//! | `public` | list | `[Register, Login]` | Commands without auth |
//!
//! ## OpenAPI Info
//!
//! | Option | Description |
//! |--------|-------------|
//! | `title` | API title for OpenAPI spec |
//! | `description` | API description (markdown) |
//! | `api_version` | Semantic version string |
//! | `license` | License name (e.g., "MIT") |
//! | `license_url` | URL to license text |
//! | `contact_name` | API maintainer name |
//! | `contact_email` | Support email address |
//! | `contact_url` | Support website URL |
//!
//! ## Deprecation
//!
//! | Option | Description |
//! |--------|-------------|
//! | `deprecated_in` | Version where API was deprecated |
//!
//! # Handler Configuration
//!
//! The `handlers` option controls CRUD handler generation:
//!
//! ```rust,ignore
//! // Generate all handlers (create, get, update, delete, list)
//! api(tag = "Users", handlers)
//!
//! // Generate specific handlers only
//! api(tag = "Users", handlers(create, get, list))
//!
//! // Disable handlers (commands only)
//! api(tag = "Users", handlers = false)
//! ```
//!
//! # Complete Example
//!
//! ```rust,ignore
//! #[entity(
//!     table = "users",
//!     api(
//!         tag = "Users",
//!         tag_description = "User account management endpoints",
//!         path_prefix = "/api",
//!         version = "v1",
//!         security = "bearer",
//!         public = [Register, Login],
//!         handlers(create, get, update, list),
//!         title = "User Service",
//!         api_version = "1.0.0",
//!         license = "MIT"
//!     )
//! )]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!     #[field(create, update, response)]
//!     pub email: String,
//! }
//! ```
//!
//! # Module Structure
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`config`] | Type definitions for `ApiConfig` and `HandlerConfig` |
//! | [`parser`] | Attribute parsing logic for `api(...)` |

mod config;
mod parser;

pub use config::ApiConfig;
pub use parser::parse_api_config;

#[cfg(test)]
mod tests;
