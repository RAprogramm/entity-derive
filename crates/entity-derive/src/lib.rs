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
pub use entity_derive_impl::Entity;
