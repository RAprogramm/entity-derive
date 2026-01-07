// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Entity-level attribute parsing.
//!
//! This module handles parsing of entity-level attributes using darling,
//! and provides the main [`EntityDef`] structure used by all code generators.
//!
//! # Module Structure
//!
//! ```text
//! entity/
//! ├── mod.rs        — Re-exports and module declarations
//! ├── def.rs        — EntityDef struct definition
//! ├── constructor.rs — EntityDef::from_derive_input()
//! ├── accessors.rs  — EntityDef accessor methods
//! ├── attrs.rs      — EntityAttrs (darling parsing struct)
//! ├── helpers.rs    — Helper parsing functions
//! ├── projection.rs — Projection definition and parsing
//! └── tests.rs      — Unit tests
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::entity::parse::EntityDef;
//!
//! let entity = EntityDef::from_derive_input(&input)?;
//!
//! // Access entity metadata
//! let table = entity.full_table_name();
//! let id_field = entity.id_field();
//!
//! // Access field categories
//! let create_fields = entity.create_fields();
//! let update_fields = entity.update_fields();
//! ```

mod accessors;
mod attrs;
mod constructor;
mod def;
mod helpers;
mod projection;

pub use attrs::EntityAttrs;
pub use def::EntityDef;
pub use projection::{ProjectionDef, parse_projection_attrs};

#[cfg(test)]
mod tests;
