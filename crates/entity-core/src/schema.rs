// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Runtime schema assertion for derived entities.
//!
//! Generated repositories build their SQL from entity metadata, so a
//! drifted table (renamed column, missed migration, nullability change)
//! surfaces as a runtime decode error instead of a compile error. This
//! module restores an equivalent guarantee with one integration test:
//! compare every entity's declared columns against
//! `information_schema.columns` and report all mismatches at once.
//!
//! ```rust,ignore
//! #[sqlx::test]
//! async fn schema_matches(pool: PgPool) {
//!     User::assert_schema(&pool).await.expect("users drifted");
//!     Parcel::assert_schema(&pool).await.expect("parcels drifted");
//! }
//! ```
//!
//! # Type comparison
//!
//! Declared SQL types are normalised before comparison (`TEXT` ↔
//! `character varying` ↔ `character` are one text family; parametrised
//! types drop their arguments; arrays compare as arrays). Columns whose
//! database type is `USER-DEFINED` (Postgres enums, domains) skip the
//! type check — the macro cannot know the enum's SQL name unless
//! `#[column(pg_enum = ...)]` is used, and presence + nullability still
//! guard those columns.

/// Declared shape of one entity column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaColumn {
    /// Column name.
    pub name: &'static str,

    /// Declared SQL type as generated for DDL (e.g. `TEXT`, `UUID`,
    /// `TIMESTAMPTZ`, `TEXT[]`).
    pub sql_type: &'static str,

    /// Whether the column accepts NULL.
    pub nullable: bool
}

/// Declared shape of an entity's table.
#[derive(Debug, Clone, Copy)]
pub struct TableSchema {
    /// Table name (unqualified).
    pub table: &'static str,

    /// Database schema the table lives in.
    pub schema: &'static str,

    /// Declared columns.
    pub columns: &'static [SchemaColumn]
}

/// One detected divergence between the entity and the live table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaDrift {
    /// The table does not exist at all.
    MissingTable,

    /// A declared column is absent from the table.
    MissingColumn {
        /// Declared column name.
        column: String
    },

    /// Column exists but NULL-ability differs.
    NullabilityMismatch {
        /// Column name.
        column:            String,
        /// Whether the entity declares the column nullable.
        declared_nullable: bool
    },

    /// Column exists but its type family differs.
    TypeMismatch {
        /// Column name.
        column:   String,
        /// Normalised declared type.
        declared: String,
        /// Type reported by `information_schema`.
        actual:   String
    }
}

impl std::fmt::Display for SchemaDrift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTable => write!(f, "table does not exist"),
            Self::MissingColumn {
                column
            } => write!(f, "column `{column}` is missing"),
            Self::NullabilityMismatch {
                column,
                declared_nullable
            } => write!(
                f,
                "column `{column}` nullability differs: entity says {}",
                if *declared_nullable {
                    "nullable"
                } else {
                    "NOT NULL"
                }
            ),
            Self::TypeMismatch {
                column,
                declared,
                actual
            } => write!(
                f,
                "column `{column}` type differs: entity says `{declared}`, database says `{actual}`"
            )
        }
    }
}

/// All divergences found for one table.
#[derive(Debug, Clone)]
pub struct SchemaMismatch {
    /// Table the drift belongs to.
    pub table: String,

    /// Every detected divergence.
    pub drifts: Vec<SchemaDrift>
}

impl std::fmt::Display for SchemaMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "schema drift in `{}`:", self.table)?;
        for drift in &self.drifts {
            writeln!(f, "  - {drift}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SchemaMismatch {}

/// Errors from the assertion itself (query failure vs. actual drift).
#[derive(Debug)]
pub enum SchemaCheckError {
    /// The `information_schema` query failed.
    Query(sqlx::Error),

    /// The table diverges from the entity declaration.
    Mismatch(SchemaMismatch)
}

impl std::fmt::Display for SchemaCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(e) => write!(f, "schema check query failed: {e}"),
            Self::Mismatch(m) => write!(f, "{m}")
        }
    }
}

impl std::error::Error for SchemaCheckError {}

impl From<sqlx::Error> for SchemaCheckError {
    fn from(e: sqlx::Error) -> Self {
        Self::Query(e)
    }
}

/// Compare a declared table shape against the live database.
///
/// Reports every divergence at once instead of failing on the first.
///
/// # Errors
///
/// [`SchemaCheckError::Query`] if `information_schema` cannot be read;
/// [`SchemaCheckError::Mismatch`] listing all drifts otherwise.
pub async fn assert_table(
    pool: &sqlx::PgPool,
    declared: &TableSchema
) -> Result<(), SchemaCheckError> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT column_name, data_type, is_nullable, udt_name \
         FROM information_schema.columns \
         WHERE table_schema = $1 AND table_name = $2"
    )
    .bind(declared.schema)
    .bind(declared.table)
    .fetch_all(pool)
    .await?;

    let mut drifts = Vec::new();
    if rows.is_empty() {
        drifts.push(SchemaDrift::MissingTable);
    } else {
        for col in declared.columns {
            let Some((_, data_type, is_nullable, _udt)) =
                rows.iter().find(|(name, ..)| name == col.name)
            else {
                drifts.push(SchemaDrift::MissingColumn {
                    column: col.name.to_string()
                });
                continue;
            };

            let db_nullable = is_nullable == "YES";
            if db_nullable != col.nullable {
                drifts.push(SchemaDrift::NullabilityMismatch {
                    column:            col.name.to_string(),
                    declared_nullable: col.nullable
                });
            }

            let declared_family = normalize(col.sql_type);
            let actual_family = normalize(data_type);
            if actual_family != "user-defined"
                && declared_family != actual_family
                && !(declared_family == "array" && actual_family == "array")
            {
                drifts.push(SchemaDrift::TypeMismatch {
                    column:   col.name.to_string(),
                    declared: declared_family,
                    actual:   actual_family
                });
            }
        }
    }

    if drifts.is_empty() {
        Ok(())
    } else {
        Err(SchemaCheckError::Mismatch(SchemaMismatch {
            table: declared.table.to_string(),
            drifts
        }))
    }
}

/// Normalise a SQL type to a comparable family name.
///
/// Drops parameters (`VARCHAR(255)` → text family), folds the text
/// family together and maps common aliases to the names
/// `information_schema.data_type` uses.
fn normalize(sql_type: &str) -> String {
    let lower = sql_type.trim().to_lowercase();
    let base = lower.split('(').next().unwrap_or(&lower).trim().to_string();

    if base.ends_with("[]") || base == "array" {
        return "array".to_string();
    }

    match base.as_str() {
        "text" | "varchar" | "character varying" | "character" | "char" | "citext" => {
            "text".to_string()
        }
        "timestamptz" | "timestamp with time zone" => "timestamp with time zone".to_string(),
        "timestamp" | "timestamp without time zone" => "timestamp without time zone".to_string(),
        "int8" | "bigint" | "bigserial" => "bigint".to_string(),
        "int4" | "int" | "integer" | "serial" => "integer".to_string(),
        "int2" | "smallint" => "smallint".to_string(),
        "float8" | "double precision" => "double precision".to_string(),
        "float4" | "real" => "real".to_string(),
        "bool" | "boolean" => "boolean".to_string(),
        other => other.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_folds_text_family() {
        assert_eq!(normalize("TEXT"), "text");
        assert_eq!(normalize("VARCHAR(255)"), "text");
        assert_eq!(normalize("character varying"), "text");
        assert_eq!(normalize("character"), "text");
    }

    #[test]
    fn normalize_maps_aliases() {
        assert_eq!(normalize("TIMESTAMPTZ"), "timestamp with time zone");
        assert_eq!(normalize("BIGINT"), "bigint");
        assert_eq!(normalize("DOUBLE PRECISION"), "double precision");
        assert_eq!(normalize("BOOLEAN"), "boolean");
        assert_eq!(normalize("uuid"), "uuid");
        assert_eq!(normalize("JSONB"), "jsonb");
    }

    #[test]
    fn normalize_detects_arrays() {
        assert_eq!(normalize("TEXT[]"), "array");
        assert_eq!(normalize("ARRAY"), "array");
    }

    #[test]
    fn mismatch_display_lists_all_drifts() {
        let m = SchemaMismatch {
            table:  "users".into(),
            drifts: vec![
                SchemaDrift::MissingColumn {
                    column: "email".into()
                },
                SchemaDrift::NullabilityMismatch {
                    column:            "name".into(),
                    declared_nullable: false
                },
            ]
        };
        let text = m.to_string();
        assert!(text.contains("schema drift in `users`"));
        assert!(text.contains("`email` is missing"));
        assert!(text.contains("`name` nullability differs"));
    }
}
