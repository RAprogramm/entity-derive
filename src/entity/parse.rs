//! Attribute parsing for the Entity derive macro.
//!
//! This module handles extraction of entity metadata from Rust attributes.
//! It uses [`darling`] for entity-level attributes and manual parsing for
//! field-level attributes (which use marker-style syntax).
//!
//! # Parsing Strategy
//!
//! Entity-level attributes like `#[entity(table = "users", schema = "core")]`
//! are parsed using darling's `FromDeriveInput` derive macro, which provides:
//!
//! - Automatic validation of required fields
//! - Default values for optional fields
//! - Clear error messages for invalid input
//!
//! Field-level attributes like `#[id]`, `#[auto]`, and `#[field(create,
//! update)]` use manual parsing because they're marker attributes that don't
//! fit darling's key-value model well.
//!
//! # Module Structure
//!
//! - [`sql_level`] - SQL generation level configuration
//! - [`field`] - Field-level attribute parsing
//! - [`entity`] - Entity-level attribute parsing and main definition
//!
//! # Data Structures
//!
//! ```text
//! EntityDef
//! ├── ident: Ident          (struct name, e.g., "User")
//! ├── vis: Visibility       (pub, pub(crate), etc.)
//! ├── table: String         (database table name)
//! ├── schema: String        (database schema)
//! ├── sql: SqlLevel         (generation level)
//! └── fields: Vec<FieldDef>
//!     └── FieldDef
//!         ├── ident: Ident      (field name)
//!         ├── ty: Type          (field type)
//!         ├── vis: Visibility   (field visibility)
//!         ├── is_id: bool       (#[id] present)
//!         ├── is_auto: bool     (#[auto] present)
//!         ├── create: bool      (in CreateRequest)
//!         ├── update: bool      (in UpdateRequest)
//!         ├── response: bool    (in Response)
//!         └── skip: bool        (excluded from DTOs)
//! ```
//!
//! # Examples
//!
//! ## Basic Parsing
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users")]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!
//!     #[field(create, response)]
//!     pub name: String,
//! }
//!
//! // Parses to:
//! // EntityDef {
//! //     ident: "User",
//! //     table: "users",
//! //     schema: "public",  // default
//! //     sql: SqlLevel::Full,  // default
//! //     fields: [
//! //         FieldDef { ident: "id", is_id: true, ... },
//! //         FieldDef { ident: "name", create: true, response: true, ... },
//! //     ]
//! // }
//! ```
//!
//! ## Custom Schema and SQL Level
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "products", schema = "inventory", sql = "trait")]
//! pub struct Product { /* ... */ }
//!
//! // Parses to:
//! // EntityDef {
//! //     table: "products",
//! //     schema: "inventory",
//! //     sql: SqlLevel::Trait,
//! //     ...
//! // }
//! ```

mod entity;
mod field;
mod sql_level;

pub use entity::EntityDef;
pub use field::FieldDef;
pub use sql_level::SqlLevel;
