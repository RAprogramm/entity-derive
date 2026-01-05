// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Insertable struct generation for INSERT operations.
//!
//! Generates an `Insertable{Name}` struct optimized for database INSERT
//! queries. This struct owns all values needed for a single INSERT operation.
//!
//! # Generated Struct
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! #[derive(Debug, Clone)]
//! pub struct InsertableUser {
//!     pub id: Uuid,
//!     pub name: String,
//!     pub email: String,
//!     pub created_at: DateTime<Utc>,
//! }
//! ```
//!
//! # Purpose
//!
//! The Insertable struct provides a clean interface for INSERT operations:
//!
//! - **Value ownership**: All fields are owned, ready for binding to SQL
//! - **Complete record**: Contains all columns needed for INSERT
//! - **Type conversion**: Generated `From<Entity>` and `From<&Entity>` impls
//!
//! # Usage in Repository
//!
//! ```rust,ignore
//! async fn create(&self, dto: CreateUserRequest) -> Result<User, Error> {
//!     let entity = User::from(dto);           // DTO → Entity (generates ID)
//!     let insertable = InsertableUser::from(&entity);  // Entity → Insertable
//!
//!     sqlx::query("INSERT INTO users (...) VALUES ($1, $2, ...)")
//!         .bind(insertable.id)
//!         .bind(insertable.name)
//!         // ...
//!         .execute(self).await?;
//!
//!     Ok(entity)
//! }
//! ```
//!
//! # Field Inclusion
//!
//! Like Row, the Insertable struct includes ALL fields:
//!
//! | Field Type | Included | Value Source |
//! |------------|----------|--------------|
//! | `#[id]` | Yes | Auto-generated UUID |
//! | `#[auto]` | Yes | `Default::default()` |
//! | `#[field(create)]` | Yes | From CreateRequest DTO |
//! | `#[field(skip)]` | Yes | `Default::default()` |

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EntityDef, SqlLevel};

/// Generates the `Insertable{Name}` struct for INSERT operations.
///
/// Returns an empty `TokenStream` if `sql = "none"` is specified,
/// as Insertable structs are only needed for database operations.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let insertable_name = entity.ident_with("Insertable", "");
    let field_defs = entity.all_fields().iter().map(|f| {
        let name = f.name();
        let ty = f.ty();
        quote! { pub #name: #ty }
    });

    quote! {
        #[derive(Debug, Clone)]
        #vis struct #insertable_name { #(#field_defs),* }
    }
}
