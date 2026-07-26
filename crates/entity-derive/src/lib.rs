// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/RAprogramm/entity-derive/main/logo.png",
    html_favicon_url = "https://raw.githubusercontent.com/RAprogramm/entity-derive/main/logo.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

//! # entity-derive
//!
//! One crate, all features. Re-exports:
//! - [`Entity`] derive macro from `entity-derive-impl`
//! - All types from `entity-core` ([`Pagination`], [`SortDirection`],
//!   [`Repository`])
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use entity_derive::{Entity, Pagination};
//!
//! #[derive(Entity)]
//! #[entity(table = "users")]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!     #[field(create, update, response)]
//!     pub name: String,
//! }
//!
//! // Use pagination
//! let page = Pagination::page(0, 25);
//! ```

// Re-export derive macro
// Re-export all core types
pub use entity_core::*;
pub use entity_derive_impl::{Entity, ValueObject};
/// Re-export of the error type generated HTTP handlers return.
///
/// Consumers of the `api` feature reach it through this path, so the
/// crate never has to appear in their own dependencies.
#[cfg(feature = "api")]
pub use masterror;
/// Re-export of the JSON runtime generated code serializes through.
///
/// Used by stream payloads, outbox rows and the examples in the OpenAPI
/// document.
#[cfg(any(feature = "api", feature = "streams", feature = "outbox"))]
pub use serde_json;
