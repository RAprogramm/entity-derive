// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Example attribute parsing for OpenAPI schemas.
//!
//! Extracts `#[example = ...]` attributes from fields for use in
//! OpenAPI schema documentation.
//!
//! # Supported Types
//!
//! | Type | Syntax | OpenAPI |
//! |------|--------|---------|
//! | String | `#[example = "text"]` | `example: "text"` |
//! | Integer | `#[example = 42]` | `example: 42` |
//! | Float | `#[example = 3.14]` | `example: 3.14` |
//! | Boolean | `#[example = true]` | `example: true` |
//!
//! # Example
//!
//! ```rust,ignore
//! #[field(create, response)]
//! #[example = "user@example.com"]
//! pub email: String,
//!
//! #[field(response)]
//! #[example = 25]
//! pub age: i32,
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::Attribute;

/// Example value for OpenAPI schema.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used for OpenAPI schema examples (#80)
pub enum ExampleValue {
    /// String example: `#[example = "text"]`.
    String(String),

    /// Integer example: `#[example = 42]`.
    Int(i64),

    /// Float example: `#[example = 3.14]`.
    Float(f64),

    /// Boolean example: `#[example = true]`.
    Bool(bool)
}

#[allow(dead_code)] // Will be used for OpenAPI schema examples (#80)
impl ExampleValue {
    /// Convert to TokenStream for code generation.
    #[must_use]
    pub fn to_tokens(&self) -> TokenStream {
        match self {
            Self::String(s) => quote! { #s },
            Self::Int(i) => quote! { #i },
            Self::Float(f) => quote! { #f },
            Self::Bool(b) => quote! { #b }
        }
    }

    /// Convert to utoipa schema attribute format.
    ///
    /// Returns `example = <value>` for use in `#[schema(...)]`.
    #[must_use]
    pub fn to_schema_attr(&self) -> TokenStream {
        let value = self.to_tokens();
        quote! { example = #value }
    }
}

/// Parse `#[example = ...]` attribute from field attributes.
///
/// Returns `Some(ExampleValue)` if the attribute is present and valid.
pub fn parse_example_attr(attrs: &[Attribute]) -> Option<ExampleValue> {
    for attr in attrs {
        if !attr.path().is_ident("example") {
            continue;
        }

        // Parse as name-value: #[example = value]
        if let syn::Meta::NameValue(meta) = &attr.meta {
            return parse_example_expr(&meta.value);
        }
    }

    None
}

/// Parse the expression part of the example attribute.
fn parse_example_expr(expr: &syn::Expr) -> Option<ExampleValue> {
    match expr {
        syn::Expr::Lit(lit_expr) => parse_example_lit(&lit_expr.lit),
        // Handle negative numbers: -42
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            if let syn::Expr::Lit(lit_expr) = &*unary.expr {
                match &lit_expr.lit {
                    syn::Lit::Int(lit) => {
                        let value: i64 = lit.base10_parse().ok()?;
                        Some(ExampleValue::Int(-value))
                    }
                    syn::Lit::Float(lit) => {
                        let value: f64 = lit.base10_parse().ok()?;
                        Some(ExampleValue::Float(-value))
                    }
                    _ => None
                }
            } else {
                None
            }
        }
        _ => None
    }
}

/// Parse a literal value into an ExampleValue.
fn parse_example_lit(lit: &syn::Lit) -> Option<ExampleValue> {
    match lit {
        syn::Lit::Str(s) => Some(ExampleValue::String(s.value())),
        syn::Lit::Int(i) => {
            let value: i64 = i.base10_parse().ok()?;
            Some(ExampleValue::Int(value))
        }
        syn::Lit::Float(f) => {
            let value: f64 = f.base10_parse().ok()?;
            Some(ExampleValue::Float(value))
        }
        syn::Lit::Bool(b) => Some(ExampleValue::Bool(b.value())),
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_attrs(input: &str) -> Vec<Attribute> {
        let item: syn::ItemStruct = syn::parse_str(input).unwrap();
        item.fields
            .iter()
            .next()
            .map(|f| f.attrs.clone())
            .unwrap_or_default()
    }

    #[test]
    fn parse_string_example() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[example = "user@example.com"]
                email: String,
            }
        "#
        );
        let example = parse_example_attr(&attrs);
        assert!(matches!(example, Some(ExampleValue::String(s)) if s == "user@example.com"));
    }

    #[test]
    fn parse_int_example() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[example = 42]
                age: i32,
            }
        "#
        );
        let example = parse_example_attr(&attrs);
        assert!(matches!(example, Some(ExampleValue::Int(42))));
    }

    #[test]
    fn parse_negative_int_example() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[example = -10]
                temperature: i32,
            }
        "#
        );
        let example = parse_example_attr(&attrs);
        assert!(matches!(example, Some(ExampleValue::Int(-10))));
    }

    #[test]
    fn parse_float_example() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[example = 99.99]
                price: f64,
            }
        "#
        );
        let example = parse_example_attr(&attrs);
        assert!(matches!(example, Some(ExampleValue::Float(f)) if (f - 99.99).abs() < 0.001));
    }

    #[test]
    fn parse_bool_example() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[example = true]
                active: bool,
            }
        "#
        );
        let example = parse_example_attr(&attrs);
        assert!(matches!(example, Some(ExampleValue::Bool(true))));
    }

    #[test]
    fn no_example_attr() {
        let attrs = parse_attrs(
            r#"
            struct Foo {
                #[field(create)]
                name: String,
            }
        "#
        );
        let example = parse_example_attr(&attrs);
        assert!(example.is_none());
    }

    #[test]
    fn to_schema_attr_string() {
        let example = ExampleValue::String("test".to_string());
        let tokens = example.to_schema_attr().to_string();
        assert!(tokens.contains("example"));
        assert!(tokens.contains("test"));
    }

    #[test]
    fn to_schema_attr_int() {
        let example = ExampleValue::Int(42);
        let tokens = example.to_schema_attr().to_string();
        assert!(tokens.contains("example"));
        assert!(tokens.contains("42"));
    }
}
