// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Authorization policy generation.
//!
//! Generates policy infrastructure for entities with `#[entity(policy)]`.
//!
//! # Generated Code
//!
//! | Type | Purpose |
//! |------|---------|
//! | `{Entity}Policy` | Trait with `can_*` authorization methods |
//! | `{Entity}AllowAllPolicy` | Default implementation allowing all |
//! | `PolicyRepository` | Wrapper enforcing policy checks |

mod allow_all;
mod trait_gen;
mod wrapper_gen;

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::EntityDef;

/// Main entry point for policy code generation.
///
/// Returns empty `TokenStream` if `policy` is not enabled.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_policy() {
        return TokenStream::new();
    }

    let policy_trait = trait_gen::generate(entity);
    let allow_all = allow_all::generate(entity);
    let wrapper = wrapper_gen::generate(entity);

    quote! {
        #policy_trait
        #allow_all
        #wrapper
    }
}
