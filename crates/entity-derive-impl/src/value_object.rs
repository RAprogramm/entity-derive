// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! `ValueObject` derive macro implementation.
//!
//! Generates trait implementations for `PostgreSQL` enum types: Display,
//! `FromStr`, `AsRef`\<str\>, and `TryFrom`\<&str\>.
//!
//! # Example
//!
//! ```rust,ignore
//! #[derive(ValueObject)]
//! #[value_object(pg_type = "order_status")]
//! pub enum OrderStatus {
//!     Pending,
//!     Confirmed,
//!     Cancelled,
//! }
//! ```
//!
//! Generates:
//! - `impl Display` — lowercase variant names
//! - `impl FromStr` — case-insensitive parsing
//! - `impl AsRef<str>` — lowercase string representation
//! - `impl TryFrom<&str>` — delegates to `FromStr`
//!
//! Users should also add:
//! - `Debug, Clone, PartialEq, Eq, PartialOrd` derives
//! - `#[sqlx(type_name = "...", rename_all = "lowercase")]` attribute
//! - `#[serde(rename_all = "lowercase")]` attribute

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, LitStr, parse_macro_input};

/// Main entry point for the `ValueObject` derive macro.
///
/// Parses the input `DeriveInput`, extracts the `pg_type` attribute,
/// and generates all required boilerplate code.
///
/// # Arguments
///
/// * `input` — Parsed derive input containing the enum definition
///
/// # Returns
///
/// Token stream with generated code, or compile error if input is invalid.
///
/// # Errors
///
/// Returns a compile error if:
/// - Input is not an enum
/// - `pg_type` attribute is missing
///
/// # Example
///
/// ```rust,ignore
/// use entity_derive::ValueObject;
///
/// #[derive(ValueObject)]
/// #[value_object(pg_type = "order_status")]
/// pub enum OrderStatus {
///     Pending,
///     Confirmed,
/// }
/// ```
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into()
    }
}

/// Generate all boilerplate code for a `ValueObject` enum.
///
/// # Arguments
///
/// * `input` — Parsed derive input containing the enum definition
///
/// # Returns
///
/// Token stream with all generated code, or an error if validation fails.
fn generate(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;

    // Ensure it's an enum
    let variants = match &input.data {
        syn::Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ValueObject can only be derived for enums"
            ));
        }
    };

    let pg_type = extract_pg_type(&input.attrs)?;
    let generate_sqlx = extract_sqlx_flag(&input.attrs);

    // Build lowercase variant names using convert_case
    let variant_names: Vec<String> = variants
        .iter()
        .map(|v| {
            let ident_str = v.ident.to_string();
            ident_str.to_case(Case::Snake)
        })
        .collect();

    // Generate Display match arms
    let display_arms: Vec<TokenStream2> = variants
        .iter()
        .zip(&variant_names)
        .map(|(v, name)| {
            let variant_ident = &v.ident;
            quote! { Self::#variant_ident => write!(f, #name) }
        })
        .collect();

    // Generate FromStr match arms (case-insensitive)
    let fromstr_arms: Vec<TokenStream2> = variants
        .iter()
        .zip(&variant_names)
        .map(|(v, name)| {
            let variant_ident = &v.ident;
            let name_lower = name.to_lowercase();
            quote! { #name_lower => Ok(Self::#variant_ident) }
        })
        .collect();

    // Generate AsRef<str> match arms
    let asref_arms: Vec<TokenStream2> = variants
        .iter()
        .zip(&variant_names)
        .map(|(v, name)| {
            let variant_ident = &v.ident;
            quote! { Self::#variant_ident => #name }
        })
        .collect();

    let create_type_sql = format!(
        "DO $$ BEGIN CREATE TYPE {} AS ENUM ({}); EXCEPTION WHEN duplicate_object THEN NULL; END $$;",
        pg_type,
        variant_names
            .iter()
            .map(|v| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let sqlx_impls = if generate_sqlx {
        quote! {
            #[cfg(feature = "postgres")]
            impl ::sqlx::Type<::sqlx::Postgres> for #name {
                fn type_info() -> ::sqlx::postgres::PgTypeInfo {
                    ::sqlx::postgres::PgTypeInfo::with_name(#pg_type)
                }
            }

            #[cfg(feature = "postgres")]
            impl<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> for #name {
                fn encode_by_ref(
                    &self,
                    buf: &mut ::sqlx::postgres::PgArgumentBuffer
                ) -> Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError> {
                    <&str as ::sqlx::Encode<'q, ::sqlx::Postgres>>::encode_by_ref(&self.as_ref(), buf)
                }
            }

            #[cfg(feature = "postgres")]
            impl<'r> ::sqlx::Decode<'r, ::sqlx::Postgres> for #name {
                fn decode(
                    value: ::sqlx::postgres::PgValueRef<'r>
                ) -> Result<Self, ::sqlx::error::BoxDynError> {
                    let s = <&str as ::sqlx::Decode<'r, ::sqlx::Postgres>>::decode(value)?;
                    s.parse().map_err(Into::into)
                }
            }
        }
    } else {
        TokenStream2::new()
    };

    Ok(quote! {
        impl #name {
            /// Postgres enum type name declared via `#[value_object(pg_type = "...")]`.
            pub const PG_TYPE: &'static str = #pg_type;

            /// Idempotent DDL creating the Postgres enum type.
            ///
            /// Safe to run repeatedly: an existing type is left untouched.
            /// Execute before `MIGRATION_UP` of entities using this enum.
            pub const PG_CREATE_TYPE: &'static str = #create_type_sql;
        }

        #sqlx_impls

        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#display_arms),*
                }
            }
        }

        impl std::str::FromStr for #name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    #(#fromstr_arms),*,
                    other => Err(format!("unknown variant `{other}`"))
                }
            }
        }

        impl AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                match self {
                    #(#asref_arms),*
                }
            }
        }

        impl std::convert::TryFrom<&str> for #name {
            type Error = String;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    })
}

/// Extract the `pg_type` value from `#[value_object(pg_type = "...")]`
/// attributes.
///
/// # Arguments
///
/// * `attrs` — Slice of attributes from the derive input
///
/// # Returns
///
/// The `pg_type` string, or an error if no `#[value_object]` attribute is
/// found.
///
/// # Errors
///
/// Returns `syn::Error` if no `#[value_object]` attribute exists.
fn extract_pg_type(attrs: &[syn::Attribute]) -> syn::Result<String> {
    let mut pg_type: Option<String> = None;

    for attr in attrs {
        if attr.path().is_ident("value_object")
            && let syn::Meta::List(meta_list) = &attr.meta
        {
            let _ = meta_list.parse_nested_meta(|meta| {
                if meta.path.is_ident("pg_type") {
                    let val_stream = meta.value()?;
                    let lit: LitStr = val_stream.parse()?;
                    pg_type = Some(lit.value());
                }
                Ok(())
            });
        }
    }

    pg_type.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "missing #[value_object(pg_type = \"...\")] attribute"
        )
    })
}

/// Whether `#[value_object(..., sqlx)]` requests generated sqlx impls.
///
/// When set, the derive emits `sqlx::Type`, `Encode` and `Decode`
/// implementations for Postgres (gated behind the user's `postgres`
/// feature). Off by default so enums that already derive `sqlx::Type`
/// keep compiling.
fn extract_sqlx_flag(attrs: &[syn::Attribute]) -> bool {
    let mut flag = false;

    for attr in attrs {
        if attr.path().is_ident("value_object")
            && let syn::Meta::List(meta_list) = &attr.meta
        {
            let _ = meta_list.parse_nested_meta(|meta| {
                if meta.path.is_ident("pg_type") {
                    let val_stream = meta.value()?;
                    let _: LitStr = val_stream.parse()?;
                } else if meta.path.is_ident("sqlx") {
                    flag = true;
                }
                Ok(())
            });
        }
    }

    flag
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_input(input: &str) -> DeriveInput {
        syn::parse_str(input).unwrap()
    }

    fn normalize(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn extract_pg_type_basic() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "order_status")]
            enum OrderStatus { Pending }
        };

        let pg_type = extract_pg_type(&input.attrs).unwrap();
        assert_eq!(pg_type, "order_status");
    }

    #[test]
    fn extract_pg_type_missing_fails() {
        let input: DeriveInput = syn::parse_quote! {
            enum OrderStatus { Pending }
        };

        let result = extract_pg_type(&input.attrs);
        assert!(result.is_err());
    }

    #[test]
    fn extract_pg_type_with_quotes() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "user_role")]
            enum UserRole { Admin }
        };

        let pg_type = extract_pg_type(&input.attrs).unwrap();
        assert_eq!(pg_type, "user_role");
    }

    #[test]
    fn generate_basic_enum() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "order_status")]
            enum OrderStatus {
                Pending,
                Confirmed,
                Cancelled,
            }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("DisplayforOrderStatus"));
        assert!(output.contains("FromStrforOrderStatus"));
        assert!(output.contains("AsRef<str>forOrderStatus"));
        assert!(output.contains("TryFrom<&str>forOrderStatus"));
    }

    #[test]
    fn display_output_lowercase() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status {
                Pending,
                Confirmed,
            }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("write!(f,\"pending\")"));
        assert!(output.contains("write!(f,\"confirmed\")"));
    }

    #[test]
    fn display_output_underscore_variant() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status {
                InProgress,
            }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        // InProgress should become "in_progress"
        assert!(output.contains("write!(f,\"in_progress\")"));
    }

    #[test]
    fn fromstr_case_insensitive() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status {
                Active,
                Inactive,
            }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("\"active\"=>Ok(Self::Active)"));
        assert!(output.contains("\"inactive\"=>Ok(Self::Inactive)"));
        assert!(output.contains("s.to_lowercase().as_str()"));
    }

    #[test]
    fn fromstr_error_unknown_variant() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status { Active }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("unknownvariant"));
    }

    #[test]
    fn asref_matches_display() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status {
                Pending,
            }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("Self::Pending=>\"pending\""));
    }

    #[test]
    fn tryfrom_delegates_to_parse() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status { Active }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("s.parse()"));
        assert!(output.contains("typeError=String"));
    }

    #[test]
    fn generate_for_non_enum_fails() {
        let input = parse_input(
            r#"
            struct NotAnEnum {
                field: String,
            }
            "#
        );

        let _result = generate(&input);
        assert!(_result.is_err());
    }

    #[test]
    fn roundtrip_display_fromstr() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "order_status")]
            enum OrderStatus {
                Pending,
                Confirmed,
                Cancelled,
            }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        // Verify Display generates lowercase names
        assert!(output.contains("write!(f,\"pending\")"));
        assert!(output.contains("write!(f,\"confirmed\")"));
        assert!(output.contains("write!(f,\"cancelled\")"));

        // Verify FromStr accepts lowercase
        assert!(output.contains("\"pending\"=>Ok(Self::Pending)"));
        assert!(output.contains("\"confirmed\"=>Ok(Self::Confirmed)"));
        assert!(output.contains("\"cancelled\"=>Ok(Self::Cancelled)"));
    }

    #[test]
    fn variant_with_numbers() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status {
                V2Active,
                V3Inactive,
            }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        // V2Active -> v_2_active, V3Inactive -> v_3_inactive
        assert!(output.contains("write!(f,\"v_2_active\")"));
        assert!(output.contains("write!(f,\"v_3_inactive\")"));
    }

    #[test]
    fn single_variant_enum() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status { Only }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("DisplayforStatus"));
        assert!(output.contains("write!(f,\"only\")"));
        assert!(output.contains("\"only\"=>Ok(Self::Only)"));
    }

    #[test]
    fn multiple_pg_type_attributes_use_last() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "first")]
            #[value_object(pg_type = "second")]
            enum Status { Active }
        };

        // darling extracts the last one, so we get "second"
        let _result = generate(&input);
        // The extract_pg_type function iterates and returns the last match
        let pg_type = extract_pg_type(&input.attrs).unwrap();
        assert_eq!(pg_type, "second");
    }

    #[test]
    fn fromstr_error_type_string() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status { Active }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("typeErr=String"));
    }

    #[test]
    fn tryfrom_error_type_string() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status { Active }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("typeError=String"));
    }

    #[test]
    fn all_traits_implemented() {
        let input = parse_input(
            r#"
            #[value_object(pg_type = "status")]
            enum Status { Active, Inactive }
            "#
        );

        let result = generate(&input).unwrap();
        let output = normalize(&result.to_string());

        assert!(output.contains("DisplayforStatus"));
        assert!(output.contains("FromStrforStatus"));
        assert!(output.contains("AsRef<str>forStatus"));
        assert!(output.contains("TryFrom<&str>forStatus"));
    }
}

#[cfg(test)]
mod pg_type_tests {
    use super::*;

    #[test]
    fn generates_pg_type_const() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "order_status")]
            enum OrderStatus { Pending, Shipped }
        };
        let code = generate(&input).unwrap().to_string();
        assert!(code.contains("PG_TYPE"));
        assert!(code.contains("\"order_status\""));
    }

    #[test]
    fn generates_idempotent_create_type() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "order_status")]
            enum OrderStatus { Pending, Shipped }
        };
        let code = generate(&input).unwrap().to_string();
        assert!(code.contains(
            "DO $$ BEGIN CREATE TYPE order_status AS ENUM ('pending', 'shipped'); \
             EXCEPTION WHEN duplicate_object THEN NULL; END $$;"
        ));
    }

    #[test]
    fn sqlx_impls_absent_by_default() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "order_status")]
            enum OrderStatus { Pending }
        };
        let code = generate(&input).unwrap().to_string();
        assert!(!code.contains("Encode"));
        assert!(!code.contains("Decode"));
    }

    #[test]
    fn sqlx_flag_generates_impls() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "order_status", sqlx)]
            enum OrderStatus { Pending }
        };
        let code = generate(&input).unwrap().to_string();
        assert!(code.contains("Encode"));
        assert!(code.contains("Decode"));
        assert!(code.contains("type_info"));
    }

    #[test]
    fn multi_word_variants_snake_cased() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "kind")]
            enum Kind { InReview, DoneDeal }
        };
        let code = generate(&input).unwrap().to_string();
        assert!(code.contains("'in_review', 'done_deal'"));
    }
}

#[cfg(test)]
mod sqlx_flag_tests {
    use super::*;

    #[test]
    fn flag_absent_returns_false() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "kind")]
            enum Kind { A }
        };
        assert!(!extract_sqlx_flag(&input.attrs));
    }

    #[test]
    fn flag_present_returns_true() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(pg_type = "kind", sqlx)]
            enum Kind { A }
        };
        assert!(extract_sqlx_flag(&input.attrs));
    }

    #[test]
    fn unrelated_attribute_ignored() {
        let input: DeriveInput = syn::parse_quote! {
            #[derive(Debug)]
            #[allow(dead_code)]
            enum Kind { A }
        };
        assert!(!extract_sqlx_flag(&input.attrs));
    }

    #[test]
    fn flag_before_pg_type_returns_true() {
        let input: DeriveInput = syn::parse_quote! {
            #[value_object(sqlx, pg_type = "kind")]
            enum Kind { A }
        };
        assert!(extract_sqlx_flag(&input.attrs));
    }
}
