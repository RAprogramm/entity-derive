// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Row-to-entity mapping configuration.
//!
//! Controls how fields are transformed when mapping from database rows
//! to entity structs in the `From<Row> for Entity` implementation.
//!
//! # Supported Mappings
//!
//! | Attribute | Generated Code | Purpose |
//! |-----------|----------------|---------|
//! | `#[map(empty_to_none)]` | `row.field.filter(\|s\| !s.is_empty())` | Empty string to `None` |
//! | `#[map(unwrap_default)]` | `row.field.unwrap_or_default()` | Unwrap with default |
//! | `#[map(now)]` | `row.field.unwrap_or_else(chrono::Utc::now)` | Use current time |
//! | `#[map(expr = "...")]` | Raw expression | Custom escape hatch |
//!
//! # Example
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users")]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!
//!     #[field(response)]
//!     #[map(empty_to_none)]
//!     pub nickname: Option<String>,
//!
//!     #[field(create, response)]
//!     #[map(unwrap_default)]
//!     pub age: Option<i32>,
//!
//!     #[field(response)]
//!     #[map(now)]
//!     pub last_seen: Option<DateTime<Utc>>,
//!
//!     #[field(response)]
//!     #[map(expr = "row.raw_score.parse().unwrap_or(0)")]
//!     pub score: i32,
//! }
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident};

/// Row-to-entity mapping configuration.
///
/// Defines how a field should be transformed when mapping from a database row
/// to the entity struct in the `From<Row> for Entity` implementation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MapConfig {
    /// No mapping transformation.
    #[default]
    None,

    /// Filter out empty strings: `.filter(|s| !s.is_empty())`.
    ///
    /// Works with `Option<String>` and `Option<&str>`.
    ///
    /// # Example
    ///
    /// ```text
    /// // Input: Some("")      → Output: None
    /// // Input: Some("hello") → Output: Some("hello")
    /// // Input: None          → Output: None
    /// ```
    EmptyToNone,

    /// Unwrap with default: `.unwrap_or_default()`.
    ///
    /// Works with any `Option<T>` where `T: Default`.
    ///
    /// # Example
    ///
    /// ```text
    /// // Input: Some(42)  → Output: 42
    /// // Input: None      → Output: T::default()
    /// ```
    UnwrapDefault,

    /// Use current time if None: `.unwrap_or_else(chrono::Utc::now)`.
    ///
    /// Specifically for `Option<DateTime<Utc>>`.
    ///
    /// # Example
    ///
    /// ```text
    /// // Input: Some(t)     → Output: t
    /// // Input: None        → Output: chrono::Utc::now()
    /// ```
    Now,

    /// Raw expression escape hatch.
    ///
    /// The expression is used verbatim in the generated code.
    /// No validation or transformation is applied.
    ///
    /// # Example
    ///
    /// ```text
    /// #[map(expr = "row.raw_value.parse().unwrap_or(0)")]
    /// pub field: i32
    /// ```
    Expr(String)
}

impl MapConfig {
    /// Parse map config from `#[map(...)]` attribute.
    ///
    /// # Recognized Options
    ///
    /// - `empty_to_none` — Filter empty strings to `None`
    /// - `unwrap_default` — Unwrap with default value
    /// - `now` — Use current UTC time if `None`
    /// - `expr = "..."` — Raw expression escape hatch
    ///
    /// # Errors
    ///
    /// Returns `None` if the attribute is not recognized (not a map attribute).
    /// Returns `Some(MapConfig)` for recognized map attributes.
    pub fn from_attr(attr: &Attribute) -> Option<Self> {
        if !attr.path().is_ident("map") {
            return None;
        }

        match &attr.meta {
            syn::Meta::List(meta_list) => {
                // Try simple ident: #[map(empty_to_none)]
                if let Ok(ident) = meta_list.parse_args::<Ident>() {
                    return Some(Self::from_ident(&ident));
                }

                // Try nested meta: #[map(empty_to_none, expr = "...")]
                let mut result = Self::None;
                let _ = meta_list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("empty_to_none") {
                        result = Self::EmptyToNone;
                    } else if meta.path.is_ident("unwrap_default") {
                        result = Self::UnwrapDefault;
                    } else if meta.path.is_ident("now") {
                        result = Self::Now;
                    } else if meta.path.is_ident("expr") {
                        let _: syn::Token![=] = meta.input.parse()?;
                        let value: syn::LitStr = meta.input.parse()?;
                        result = Self::Expr(value.value());
                    }
                    Ok(())
                });

                Some(result)
            }
            syn::Meta::NameValue(nv) => {
                if nv.path.is_ident("expr")
                    && let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) = &nv.value
                {
                    return Some(Self::Expr(lit_str.value()));
                }
                Some(Self::None)
            }
            syn::Meta::Path(_) => Some(Self::None)
        }
    }

    /// Parse map config from an identifier.
    ///
    /// # Recognized Identifiers
    ///
    /// - `"empty_to_none"` → `MapConfig::EmptyToNone`
    /// - `"unwrap_default"` → `MapConfig::UnwrapDefault`
    /// - `"now"` → `MapConfig::Now`
    /// - Any other → `MapConfig::None`
    #[must_use]
    pub fn from_ident(ident: &Ident) -> Self {
        match ident.to_string().as_str() {
            "empty_to_none" => Self::EmptyToNone,
            "unwrap_default" => Self::UnwrapDefault,
            "now" => Self::Now,
            _ => Self::None
        }
    }

    /// Generate the token stream for this mapping config.
    ///
    /// # Arguments
    ///
    /// * `field_name` — The field identifier on the source struct (e.g.,
    ///   `nickname`)
    /// * `source_name` — The source variable name (e.g., `row`)
    ///
    /// # Returns
    ///
    /// A `TokenStream` representing the transformation expression.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = MapConfig::EmptyToNone;
    /// let tokens = config.generate("nickname", "row");
    /// // Generates: row.nickname.filter(|s| !s.is_empty())
    /// ```
    #[must_use]
    pub fn generate(&self, field_name: &Ident, source_name: &Ident) -> TokenStream {
        match self {
            Self::None => quote! { #source_name.#field_name },
            Self::EmptyToNone => {
                quote! { #source_name.#field_name.filter(|s| !s.is_empty()) }
            }
            Self::UnwrapDefault => {
                quote! { Some(#source_name.#field_name.unwrap_or_default()) }
            }
            Self::Now => {
                quote! { Some(#source_name.#field_name.unwrap_or_else(chrono::Utc::now)) }
            }
            Self::Expr(expr) => {
                let expr_tokens: TokenStream = expr.parse().unwrap_or_default();
                quote! { #expr_tokens }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    fn parse_map_attr(tokens: proc_macro2::TokenStream) -> MapConfig {
        let attr: Attribute = parse_quote!(#[map(#tokens)]);
        MapConfig::from_attr(&attr).unwrap_or_default()
    }

    #[test]
    fn default_is_none() {
        let config = MapConfig::default();
        assert!(matches!(config, MapConfig::None));
    }

    #[test]
    fn parse_empty_to_none() {
        let config = parse_map_attr(quote! { empty_to_none });
        assert!(matches!(config, MapConfig::EmptyToNone));
    }

    #[test]
    fn parse_unwrap_default() {
        let config = parse_map_attr(quote! { unwrap_default });
        assert!(matches!(config, MapConfig::UnwrapDefault));
    }

    #[test]
    fn parse_now() {
        let config = parse_map_attr(quote! { now });
        assert!(matches!(config, MapConfig::Now));
    }

    #[test]
    fn parse_expr() {
        let config = parse_map_attr(quote! { expr = "row.score.parse().unwrap_or(0)" });
        assert!(matches!(config, MapConfig::Expr(s) if s == "row.score.parse().unwrap_or(0)"));
    }

    #[test]
    fn parse_unknown_is_none() {
        let config = parse_map_attr(quote! { unknown_option });
        assert!(matches!(config, MapConfig::None));
    }

    #[test]
    fn from_ident_empty_to_none() {
        let ident = Ident::new("empty_to_none", proc_macro2::Span::call_site());
        assert!(matches!(
            MapConfig::from_ident(&ident),
            MapConfig::EmptyToNone
        ));
    }

    #[test]
    fn from_ident_unwrap_default() {
        let ident = Ident::new("unwrap_default", proc_macro2::Span::call_site());
        assert!(matches!(
            MapConfig::from_ident(&ident),
            MapConfig::UnwrapDefault
        ));
    }

    #[test]
    fn from_ident_now() {
        let ident = Ident::new("now", proc_macro2::Span::call_site());
        assert!(matches!(MapConfig::from_ident(&ident), MapConfig::Now));
    }

    #[test]
    fn from_ident_unknown() {
        let ident = Ident::new("unknown", proc_macro2::Span::call_site());
        assert!(matches!(MapConfig::from_ident(&ident), MapConfig::None));
    }

    #[test]
    fn generate_none() {
        let config = MapConfig::None;
        let field = Ident::new("name", proc_macro2::Span::call_site());
        let source = Ident::new("row", proc_macro2::Span::call_site());
        let tokens = config.generate(&field, &source);
        let expected = quote! { row.name };
        assert_eq!(tokens.to_string(), expected.to_string());
    }

    #[test]
    fn generate_empty_to_none() {
        let config = MapConfig::EmptyToNone;
        let field = Ident::new("nickname", proc_macro2::Span::call_site());
        let source = Ident::new("row", proc_macro2::Span::call_site());
        let tokens = config.generate(&field, &source);
        let expected = quote! { row.nickname.filter(|s| !s.is_empty()) };
        assert_eq!(tokens.to_string(), expected.to_string());
    }

    #[test]
    fn generate_unwrap_default() {
        let config = MapConfig::UnwrapDefault;
        let field = Ident::new("age", proc_macro2::Span::call_site());
        let source = Ident::new("row", proc_macro2::Span::call_site());
        let tokens = config.generate(&field, &source);
        let expected = quote! { Some(row.age.unwrap_or_default()) };
        assert_eq!(tokens.to_string(), expected.to_string());
    }

    #[test]
    fn generate_now() {
        let config = MapConfig::Now;
        let field = Ident::new("last_seen", proc_macro2::Span::call_site());
        let source = Ident::new("row", proc_macro2::Span::call_site());
        let tokens = config.generate(&field, &source);
        let expected = quote! { Some(row.last_seen.unwrap_or_else(chrono::Utc::now)) };
        assert_eq!(tokens.to_string(), expected.to_string());
    }

    #[test]
    fn generate_expr() {
        let config = MapConfig::Expr("row.raw.parse().unwrap_or(0)".to_string());
        let field = Ident::new("score", proc_macro2::Span::call_site());
        let source = Ident::new("row", proc_macro2::Span::call_site());
        let tokens = config.generate(&field, &source);
        let expected = quote! { row.raw.parse().unwrap_or(0) };
        assert_eq!(tokens.to_string(), expected.to_string());
    }

    #[test]
    fn not_a_map_attribute() {
        let attr: Attribute = parse_quote!(#[field(create)]);
        assert!(MapConfig::from_attr(&attr).is_none());
    }

    #[test]
    fn from_attr_namevalue_expr() {
        let attr: Attribute = parse_quote!(#[map(expr = "row.raw.parse()")]);
        let config = MapConfig::from_attr(&attr);
        assert!(matches!(config, Some(MapConfig::Expr(s)) if s == "row.raw.parse()"));
    }

    #[test]
    fn from_attr_path_only() {
        let attr: Attribute = parse_quote!(#[map]);
        let config = MapConfig::from_attr(&attr);
        assert!(matches!(config, Some(MapConfig::None)));
    }

    #[test]
    fn generate_unwrap_default_with_some() {
        let config = MapConfig::UnwrapDefault;
        let field = Ident::new("value", proc_macro2::Span::call_site());
        let source = Ident::new("row", proc_macro2::Span::call_site());
        let tokens = config.generate(&field, &source);
        let tokens_str = tokens.to_string();
        assert!(tokens_str.contains("Some"));
        assert!(tokens_str.contains("unwrap_or_default"));
    }

    #[test]
    fn generate_now_with_some() {
        let config = MapConfig::Now;
        let field = Ident::new("ts", proc_macro2::Span::call_site());
        let source = Ident::new("row", proc_macro2::Span::call_site());
        let tokens = config.generate(&field, &source);
        let tokens_str = tokens.to_string();
        assert!(tokens_str.contains("Some"));
        assert!(tokens_str.contains("unwrap_or_else"));
    }

    #[test]
    fn from_ident_with_numbers() {
        let ident = Ident::new("empty_to_none", proc_macro2::Span::call_site());
        assert!(matches!(
            MapConfig::from_ident(&ident),
            MapConfig::EmptyToNone
        ));

        let ident2 = Ident::new("unwrap_default", proc_macro2::Span::call_site());
        assert!(matches!(
            MapConfig::from_ident(&ident2),
            MapConfig::UnwrapDefault
        ));

        let ident3 = Ident::new("now", proc_macro2::Span::call_site());
        assert!(matches!(MapConfig::from_ident(&ident3), MapConfig::Now));
    }

    #[test]
    fn parse_expr_with_spaces() {
        let config = parse_map_attr(quote! { expr = "row.value.parse().unwrap_or(0)" });
        assert!(matches!(config, MapConfig::Expr(s) if s.contains("parse")));
    }

    #[test]
    fn generate_expr_complex() {
        let config = MapConfig::Expr("row.custom.parse::<i32>().unwrap_or(42)".to_string());
        let field = Ident::new("score", proc_macro2::Span::call_site());
        let source = Ident::new("row", proc_macro2::Span::call_site());
        let tokens = config.generate(&field, &source);
        let tokens_str = tokens.to_string();
        assert!(tokens_str.contains("unwrap_or"));
    }
}
