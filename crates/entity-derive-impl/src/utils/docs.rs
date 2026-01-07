// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Documentation extraction utilities.
//!
//! Extracts doc comments from Rust attributes for use in OpenAPI descriptions.
//!
//! # Doc Comment Format
//!
//! In Rust, doc comments (`///` and `/** */`) are stored as `#[doc = "..."]`
//! attributes. This module extracts and cleans those comments for use in
//! OpenAPI documentation.
//!
//! # Example
//!
//! ```rust,ignore
//! /// User account entity.
//! ///
//! /// Represents a registered user in the system.
//! #[derive(Entity)]
//! pub struct User { ... }
//!
//! // Extracts to: "User account entity.\n\nRepresents a registered user..."
//! ```

use syn::Attribute;

/// Extract doc comments from attributes.
///
/// Combines all `#[doc = "..."]` attributes into a single string,
/// trimming leading whitespace from each line.
///
/// # Arguments
///
/// * `attrs` - Slice of syn Attributes
///
/// # Returns
///
/// Combined doc string, or `None` if no doc comments present.
///
/// # Example
///
/// ```rust,ignore
/// let docs = extract_doc_comments(&field.attrs);
/// if let Some(description) = docs {
///     // Use description in OpenAPI
/// }
/// ```
pub fn extract_doc_comments(attrs: &[Attribute]) -> Option<String> {
    let doc_lines: Vec<String> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            if let syn::Meta::NameValue(meta) = &attr.meta
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &meta.value
            {
                return Some(lit_str.value());
            }
            None
        })
        .collect();

    if doc_lines.is_empty() {
        return None;
    }

    // Join lines and clean up
    let combined = doc_lines
        .iter()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n");

    // Trim the result and return if non-empty
    let trimmed = combined.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Extract the first line of doc comments (summary).
///
/// Returns just the first non-empty line for use as a brief description.
///
/// # Arguments
///
/// * `attrs` - Slice of syn Attributes
///
/// # Returns
///
/// First doc line, or `None` if no doc comments present.
#[allow(dead_code)] // Will be used for endpoint summaries (#78)
pub fn extract_doc_summary(attrs: &[Attribute]) -> Option<String> {
    extract_doc_comments(attrs).and_then(|docs| {
        docs.lines()
            .find(|line| !line.trim().is_empty())
            .map(|s| s.trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_attrs(input: &str) -> Vec<Attribute> {
        let item: syn::ItemStruct = syn::parse_str(input).unwrap();
        item.attrs
    }

    #[test]
    fn extract_single_line_doc() {
        let attrs = parse_attrs(
            r#"
            /// User entity.
            struct Foo;
        "#
        );
        let docs = extract_doc_comments(&attrs);
        assert_eq!(docs, Some("User entity.".to_string()));
    }

    #[test]
    fn extract_multi_line_doc() {
        let attrs = parse_attrs(
            r#"
            /// First line.
            /// Second line.
            struct Foo;
        "#
        );
        let docs = extract_doc_comments(&attrs);
        assert_eq!(docs, Some("First line.\nSecond line.".to_string()));
    }

    #[test]
    fn extract_doc_with_empty_lines() {
        let attrs = parse_attrs(
            r#"
            /// Summary.
            ///
            /// Details here.
            struct Foo;
        "#
        );
        let docs = extract_doc_comments(&attrs);
        assert_eq!(docs, Some("Summary.\n\nDetails here.".to_string()));
    }

    #[test]
    fn extract_no_docs() {
        let attrs = parse_attrs(
            r#"
            #[derive(Debug)]
            struct Foo;
        "#
        );
        let docs = extract_doc_comments(&attrs);
        assert_eq!(docs, None);
    }

    #[test]
    fn extract_summary_only() {
        let attrs = parse_attrs(
            r#"
            /// First line summary.
            /// More details.
            struct Foo;
        "#
        );
        let summary = extract_doc_summary(&attrs);
        assert_eq!(summary, Some("First line summary.".to_string()));
    }

    #[test]
    fn extract_summary_skips_empty_first_line() {
        let attrs = parse_attrs(
            r#"
            ///
            /// Actual summary.
            struct Foo;
        "#
        );
        let summary = extract_doc_summary(&attrs);
        assert_eq!(summary, Some("Actual summary.".to_string()));
    }
}
