// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Projection definition and parsing.
//!
//! Projections define partial views of an entity, allowing optimized SELECT
//! queries that only fetch the needed columns. This is useful for APIs that
//! need different levels of detail for different use cases.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    Projection System                                │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Attribute Syntax                                                   │
//! │                                                                     │
//! │  #[projection(Public: id, name, avatar)]                           │
//! │  #[projection(Admin: id, name, email, role, created_at)]           │
//! │         │           │    └─ field list                              │
//! │         │           └────── colon separator                         │
//! │         └──────────────── projection name                           │
//! │                                                                     │
//! │  Generated Code                                                     │
//! │                                                                     │
//! │  ┌─────────────────┐   ┌─────────────────┐                         │
//! │  │   UserPublic    │   │    UserAdmin    │                         │
//! │  │ ├── id: Uuid    │   │ ├── id: Uuid    │                         │
//! │  │ ├── name: String│   │ ├── name: String│                         │
//! │  │ └── avatar: Url │   │ ├── email: String│                        │
//! │  └─────────────────┘   │ ├── role: Role  │                         │
//! │                        │ └── created_at  │                         │
//! │                        └─────────────────┘                         │
//! │                                                                     │
//! │  Repository Methods                                                 │
//! │                                                                     │
//! │  repo.find_by_id_public(id)  → UserPublic                          │
//! │  repo.find_by_id_admin(id)   → UserAdmin                           │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Use Cases
//!
//! | Projection | Use Case |
//! |------------|----------|
//! | `Public` | User-facing API responses (no sensitive data) |
//! | `Admin` | Admin panel with full details |
//! | `List` | Minimal fields for list views |
//! | `Detail` | Extended fields for detail views |
//!
//! # Generated Code
//!
//! Each projection generates:
//! - A struct with the specified fields (e.g., `UserPublic`)
//! - A `From<Entity>` implementation for conversion
//! - A `find_by_id_{name}` repository method with optimized SELECT

use syn::{Attribute, Ident};

/// A projection definition parsed from `#[projection(Name: field1, field2)]`.
///
/// # Fields
///
/// | Field | Description |
/// |-------|-------------|
/// | `name` | Projection name (e.g., `Public`, `Admin`) |
/// | `fields` | List of field names to include in this projection |
///
/// # Example
///
/// For `#[projection(Public: id, name)]`:
/// ```rust,ignore
/// ProjectionDef {
///     name: Ident("Public"),
///     fields: vec![Ident("id"), Ident("name")]
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ProjectionDef {
    /// Projection name (e.g., `Public`, `Admin`).
    pub name: Ident,

    /// List of field names to include.
    pub fields: Vec<Ident>
}

/// Parse `#[projection(Name: field1, field2, ...)]` attributes.
///
/// Extracts all projection definitions from the struct's attributes.
///
/// # Arguments
///
/// * `attrs` - Slice of syn Attributes from the struct
///
/// # Returns
///
/// Vector of parsed projection definitions.
///
/// # Syntax
///
/// Each projection attribute must have the format:
/// ```text
/// #[projection(Name: field1, field2, field3)]
/// ```
///
/// Where:
/// - `Name` is a valid Rust identifier (the projection suffix)
/// - `:` separates the name from fields
/// - Fields are comma-separated identifiers
pub fn parse_projection_attrs(attrs: &[Attribute]) -> Vec<ProjectionDef> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("projection"))
        .filter_map(|attr| {
            attr.parse_args_with(|input: syn::parse::ParseStream<'_>| {
                let name: Ident = input.parse()?;
                let _: syn::Token![:] = input.parse()?;
                let fields =
                    syn::punctuated::Punctuated::<Ident, syn::Token![,]>::parse_terminated(input)?;
                Ok(ProjectionDef {
                    name,
                    fields: fields.into_iter().collect()
                })
            })
            .ok()
        })
        .collect()
}
