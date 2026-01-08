// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! PostgreSQL type mapping.
//!
//! Maps Rust types to PostgreSQL types for migration generation.
//!
//! # Type Mapping Table
//!
//! | Rust Type | PostgreSQL Type | Notes |
//! |-----------|-----------------|-------|
//! | `Uuid` | `UUID` | |
//! | `String` | `TEXT` | Or `VARCHAR(n)` with `#[column(varchar = n)]` |
//! | `i16` | `SMALLINT` | |
//! | `i32` | `INTEGER` | |
//! | `i64` | `BIGINT` | |
//! | `f32` | `REAL` | |
//! | `f64` | `DOUBLE PRECISION` | |
//! | `bool` | `BOOLEAN` | |
//! | `DateTime<Utc>` | `TIMESTAMPTZ` | |
//! | `NaiveDate` | `DATE` | |
//! | `NaiveTime` | `TIME` | |
//! | `NaiveDateTime` | `TIMESTAMP` | |
//! | `Option<T>` | `T` | Nullable |
//! | `Vec<T>` | `T[]` | PostgreSQL array |
//! | `serde_json::Value` | `JSONB` | |
//! | `Decimal` | `DECIMAL` | |
//! | `IpAddr` | `INET` | |

use syn::Type;

use super::{SqlType, TypeMapper};
use crate::entity::parse::ColumnConfig;

/// PostgreSQL type mapper.
///
/// Converts Rust types to PostgreSQL SQL types with full support for:
/// - Primitive types (integers, floats, booleans)
/// - String types with optional VARCHAR length
/// - Date/time types from chrono
/// - UUID from uuid crate
/// - JSON from serde_json
/// - Arrays via Vec<T>
/// - Nullable types via Option<T>
pub struct PostgresTypeMapper;

impl TypeMapper for PostgresTypeMapper {
    fn map_type(&self, ty: &Type, column: &ColumnConfig) -> SqlType {
        // Handle explicit SQL type override
        if let Some(ref explicit) = column.sql_type {
            return SqlType {
                name:      explicit.clone(),
                nullable:  is_option(ty) || column.nullable,
                array_dim: 0
            };
        }

        // Handle Option<T>
        if let Some(inner) = extract_option_inner(ty) {
            let mut result = self.map_type(inner, column);
            result.nullable = true;
            return result;
        }

        // Handle Vec<T> (PostgreSQL arrays)
        if let Some(inner) = extract_vec_inner(ty) {
            let mut result = self.map_type(inner, column);
            result.array_dim += 1;
            return result;
        }

        // Map core types
        let name = map_type_name(ty, column);

        SqlType {
            name,
            nullable: column.nullable,
            array_dim: 0
        }
    }
}

/// Map a Rust type path to PostgreSQL type name.
fn map_type_name(ty: &Type, column: &ColumnConfig) -> String {
    let type_str = type_path_string(ty);

    match type_str.as_str() {
        // UUIDs
        "Uuid" | "uuid::Uuid" => "UUID".to_string(),

        // Strings
        "String" | "str" => {
            if let Some(len) = column.varchar {
                format!("VARCHAR({})", len)
            } else {
                "TEXT".to_string()
            }
        }

        // Integers
        "i8" => "SMALLINT".to_string(), // PostgreSQL has no TINYINT
        "i16" => "SMALLINT".to_string(),
        "i32" => "INTEGER".to_string(),
        "i64" => "BIGINT".to_string(),
        "u8" => "SMALLINT".to_string(),
        "u16" => "INTEGER".to_string(),
        "u32" => "BIGINT".to_string(),
        "u64" => "BIGINT".to_string(), // May overflow

        // Floats
        "f32" => "REAL".to_string(),
        "f64" => "DOUBLE PRECISION".to_string(),

        // Boolean
        "bool" => "BOOLEAN".to_string(),

        // Date/Time (chrono)
        "DateTime" | "chrono::DateTime" => "TIMESTAMPTZ".to_string(),
        "NaiveDate" | "chrono::NaiveDate" => "DATE".to_string(),
        "NaiveTime" | "chrono::NaiveTime" => "TIME".to_string(),
        "NaiveDateTime" | "chrono::NaiveDateTime" => "TIMESTAMP".to_string(),

        // JSON
        "Value" | "serde_json::Value" | "Json" | "sqlx::types::Json" => "JSONB".to_string(),

        // Decimal
        "Decimal" | "rust_decimal::Decimal" | "BigDecimal" | "bigdecimal::BigDecimal" => {
            "DECIMAL".to_string()
        }

        // Network
        "IpAddr" | "std::net::IpAddr" | "Ipv4Addr" | "Ipv6Addr" => "INET".to_string(),
        "MacAddr" => "MACADDR".to_string(),

        // Binary
        "Vec<u8>" | "bytes::Bytes" => "BYTEA".to_string(),

        // Fallback to TEXT for unknown types
        _ => "TEXT".to_string()
    }
}

/// Extract the type path as a string.
fn type_path_string(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        type_path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    } else {
        String::new()
    }
}

/// Check if a type is Option<T>.
fn is_option(ty: &Type) -> bool {
    extract_option_inner(ty).is_some()
}

/// Extract the inner type from Option<T>.
fn extract_option_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
}

/// Extract the inner type from Vec<T>.
fn extract_vec_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Vec"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn map_type(ty_tokens: proc_macro2::TokenStream) -> SqlType {
        let ty: Type = parse_quote!(#ty_tokens);
        let column = ColumnConfig::default();
        PostgresTypeMapper.map_type(&ty, &column)
    }

    fn map_type_with_column(ty_tokens: proc_macro2::TokenStream, column: ColumnConfig) -> SqlType {
        let ty: Type = parse_quote!(#ty_tokens);
        PostgresTypeMapper.map_type(&ty, &column)
    }

    #[test]
    fn map_uuid() {
        let ty = map_type(quote::quote! { Uuid });
        assert_eq!(ty.name, "UUID");
        assert!(!ty.nullable);
    }

    #[test]
    fn map_string() {
        let ty = map_type(quote::quote! { String });
        assert_eq!(ty.name, "TEXT");
    }

    #[test]
    fn map_string_varchar() {
        let mut column = ColumnConfig::default();
        column.varchar = Some(255);
        let ty = map_type_with_column(quote::quote! { String }, column);
        assert_eq!(ty.name, "VARCHAR(255)");
    }

    #[test]
    fn map_integers() {
        assert_eq!(map_type(quote::quote! { i16 }).name, "SMALLINT");
        assert_eq!(map_type(quote::quote! { i32 }).name, "INTEGER");
        assert_eq!(map_type(quote::quote! { i64 }).name, "BIGINT");
    }

    #[test]
    fn map_floats() {
        assert_eq!(map_type(quote::quote! { f32 }).name, "REAL");
        assert_eq!(map_type(quote::quote! { f64 }).name, "DOUBLE PRECISION");
    }

    #[test]
    fn map_bool() {
        assert_eq!(map_type(quote::quote! { bool }).name, "BOOLEAN");
    }

    #[test]
    fn map_datetime() {
        let ty = map_type(quote::quote! { DateTime<Utc> });
        assert_eq!(ty.name, "TIMESTAMPTZ");
    }

    #[test]
    fn map_naive_date() {
        assert_eq!(map_type(quote::quote! { NaiveDate }).name, "DATE");
    }

    #[test]
    fn map_option_nullable() {
        let ty = map_type(quote::quote! { Option<String> });
        assert_eq!(ty.name, "TEXT");
        assert!(ty.nullable);
    }

    #[test]
    fn map_vec_to_array() {
        let ty = map_type(quote::quote! { Vec<String> });
        assert_eq!(ty.name, "TEXT");
        assert_eq!(ty.array_dim, 1);
        assert_eq!(ty.to_sql_string(), "TEXT[]");
    }

    #[test]
    fn map_vec_option() {
        let ty = map_type(quote::quote! { Vec<Option<i32>> });
        assert_eq!(ty.name, "INTEGER");
        assert!(ty.nullable);
        assert_eq!(ty.array_dim, 1);
    }

    #[test]
    fn map_option_vec() {
        let ty = map_type(quote::quote! { Option<Vec<i32>> });
        assert_eq!(ty.name, "INTEGER");
        assert!(ty.nullable);
        assert_eq!(ty.array_dim, 1);
    }

    #[test]
    fn map_json() {
        assert_eq!(map_type(quote::quote! { serde_json::Value }).name, "JSONB");
    }

    #[test]
    fn map_explicit_sql_type() {
        let mut column = ColumnConfig::default();
        column.sql_type = Some("CITEXT".to_string());
        let ty = map_type_with_column(quote::quote! { String }, column);
        assert_eq!(ty.name, "CITEXT");
    }

    #[test]
    fn map_decimal() {
        assert_eq!(map_type(quote::quote! { Decimal }).name, "DECIMAL");
    }

    #[test]
    fn map_ip_addr() {
        assert_eq!(map_type(quote::quote! { IpAddr }).name, "INET");
    }

    #[test]
    fn map_unknown_to_text() {
        assert_eq!(map_type(quote::quote! { MyCustomType }).name, "TEXT");
    }
}
