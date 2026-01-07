// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

#![allow(dead_code)] // Methods used by handler generation (#77)

//! API configuration parsing for OpenAPI/utoipa integration.
//!
//! This module handles parsing of `#[entity(api(...))]` attributes for
//! automatic HTTP handler generation with OpenAPI documentation.
//!
//! # Syntax
//!
//! ```rust,ignore
//! #[entity(api(
//!     tag = "Users",                    // OpenAPI tag name (required)
//!     tag_description = "...",          // Tag description (optional)
//!     path_prefix = "/api/v1",          // URL prefix (optional)
//!     security = "bearer",              // Default security scheme (optional)
//!     public = [Register, Login],       // Commands without auth (optional)
//! ))]
//! ```
//!
//! # Generated Output
//!
//! When `api(...)` is present, the macro generates:
//! - Axum handlers with `#[utoipa::path]` annotations
//! - OpenAPI schemas via `#[derive(ToSchema)]`
//! - Router factory function
//! - OpenApi struct for Swagger UI

mod config;
mod parser;

pub use config::ApiConfig;
pub use parser::parse_api_config;

#[cfg(test)]
mod tests;
