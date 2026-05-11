// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Optional `tracing` instrumentation for generated entity methods.
//!
//! Emits a `#[cfg_attr(feature = "tracing", ::tracing::instrument(...))]`
//! attribute that wraps generated async methods. The attribute is gated
//! on the consumer crate's `tracing` feature, so users who don't depend
//! on `tracing` see zero overhead.
//!
//! # Generated Span
//!
//! Each method is wrapped with a span carrying two fields:
//!
//! - `entity = "<EntityName>"` — the entity struct name
//! - `op = "<operation>"` — a stable label per generated method (e.g.
//!   `"create"`, `"find_by_id"`, `"find_by_email"`)
//!
//! `skip_all` hides parameter values (DTOs, pool handles) from the span.
//! `err` automatically emits an `ERROR` event when the method returns
//! `Err`, with the formatted error in the message.
//!
//! # Example Output
//!
//! ```rust,ignore
//! #[cfg_attr(
//!     feature = "tracing",
//!     ::tracing::instrument(
//!         skip_all,
//!         fields(entity = "User", op = "create"),
//!         err,
//!     )
//! )]
//! pub async fn create(&self, dto: CreateUserRequest) -> Result<User, sqlx::Error> { … }
//! ```

use proc_macro2::TokenStream;
use quote::quote;

/// Generate a `#[cfg_attr(feature = "tracing", …instrument…)]` attribute.
///
/// # Arguments
///
/// * `entity_name` — entity struct name, e.g. `"User"`. Goes into the `entity =
///   …` span field as a string literal.
/// * `op` — short stable operation label, e.g. `"create"` or `"find_by_email"`.
///   Goes into the `op = …` span field.
///
/// # Returns
///
/// A `TokenStream` ready to splice in front of an `async fn`. If the
/// consumer crate does not enable the `tracing` feature, the attribute
/// expands to nothing.
#[must_use]
pub fn instrument(entity_name: &str, op: &str) -> TokenStream {
    quote! {
        #[cfg_attr(
            feature = "tracing",
            ::tracing::instrument(
                skip_all,
                fields(entity = #entity_name, op = #op),
                err(Debug)
            )
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_cfg_attr_gate() {
        let tokens = instrument("User", "create").to_string();
        assert!(
            tokens.contains("cfg_attr"),
            "must be feature-gated: {tokens}"
        );
        assert!(
            tokens.contains("feature = \"tracing\""),
            "wrong gate: {tokens}"
        );
    }

    #[test]
    fn carries_entity_and_op_fields() {
        let tokens = instrument("User", "find_by_email").to_string();
        assert!(
            tokens.contains("entity = \"User\""),
            "entity field missing: {tokens}"
        );
        assert!(
            tokens.contains("op = \"find_by_email\""),
            "op field missing: {tokens}"
        );
    }

    #[test]
    fn enables_err_event() {
        let tokens = instrument("User", "create").to_string();
        assert!(tokens.contains("err"), "err event missing: {tokens}");
    }

    #[test]
    fn skips_all_args_by_default() {
        let tokens = instrument("User", "create").to_string();
        assert!(tokens.contains("skip_all"), "skip_all missing: {tokens}");
    }

    #[test]
    fn references_tracing_via_absolute_path() {
        // `::tracing::instrument` resolves from the consumer crate's root,
        // not from entity-derive-impl. This way the user's Cargo.toml
        // owns the `tracing` dep.
        let tokens = instrument("User", "create").to_string();
        assert!(
            tokens.contains(":: tracing :: instrument"),
            "must use absolute path: {tokens}"
        );
    }
}
