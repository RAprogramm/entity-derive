// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! DDL (Data Definition Language) generation for PostgreSQL.
//!
//! Generates CREATE TABLE, CREATE INDEX, and DROP TABLE statements.

use convert_case::{Case, Casing};

use crate::entity::{
    migrations::types::{PostgresTypeMapper, TypeMapper},
    parse::{CompositeIndexDef, EntityDef, FieldDef}
};

/// Generate the complete UP migration SQL.
///
/// Includes:
/// - CREATE TABLE with columns and constraints
/// - CREATE INDEX for single-column indexes
/// - CREATE INDEX for composite indexes
pub fn generate_up(entity: &EntityDef) -> String {
    let mut sql = String::new();

    // CREATE TABLE
    sql.push_str(&generate_create_table(entity));

    // Single-column indexes
    for field in entity.all_fields() {
        if field.column().has_index() {
            sql.push_str(&generate_single_index(entity, field));
        }
    }

    // Composite indexes
    for idx in &entity.indexes {
        sql.push_str(&generate_composite_index(entity, idx));
    }

    sql
}

/// Generate the DOWN migration SQL.
pub fn generate_down(entity: &EntityDef) -> String {
    format!(
        "DROP TABLE IF EXISTS {} CASCADE;\n",
        entity.full_table_name()
    )
}

/// Generate CREATE TABLE statement.
fn generate_create_table(entity: &EntityDef) -> String {
    let mapper = PostgresTypeMapper;
    let full_table = entity.full_table_name();

    let columns: Vec<String> = entity
        .all_fields()
        .iter()
        .map(|f| generate_column_def(f, &mapper, entity))
        .collect();

    format!(
        "CREATE TABLE IF NOT EXISTS {} (\n{}\n);\n",
        full_table,
        columns.join(",\n")
    )
}

/// Generate a single column definition.
fn generate_column_def(
    field: &FieldDef,
    mapper: &PostgresTypeMapper,
    entity: &EntityDef
) -> String {
    let column_name = field.column_name();
    let sql_type = mapper.map_type(field.ty(), field.column());

    let mut parts = vec![format!("    {}", column_name)];

    // Type with array suffix
    parts.push(sql_type.to_sql_string());

    // PRIMARY KEY for #[id] fields
    if field.is_id() {
        parts.push("PRIMARY KEY".to_string());
    } else if !sql_type.nullable {
        // NOT NULL unless nullable
        parts.push("NOT NULL".to_string());
    }

    // UNIQUE constraint
    if field.is_unique() {
        parts.push("UNIQUE".to_string());
    }

    // DEFAULT value
    if let Some(ref default) = field.column().default {
        parts.push(format!("DEFAULT {}", default));
    }

    // CHECK constraint
    if let Some(ref check) = field.column().check {
        parts.push(format!("CHECK ({})", check));
    }

    // Foreign key REFERENCES from #[belongs_to]
    if field.is_relation()
        && let Some(parent) = field.belongs_to()
    {
        let parent_table = parent.to_string().to_case(Case::Snake);
        // Use same schema as current entity for the reference
        let ref_table = format!("{}.{}", entity.schema, pluralize(&parent_table));
        let mut fk_str = format!("REFERENCES {}(id)", ref_table);

        if let Some(action) = &field.storage.on_delete {
            fk_str.push_str(&format!(" ON DELETE {}", action.as_sql()));
        }

        parts.push(fk_str);
    }

    parts.join(" ")
}

/// Generate CREATE INDEX for a single column.
fn generate_single_index(entity: &EntityDef, field: &FieldDef) -> String {
    let table = &entity.table;
    let schema = &entity.schema;
    let column = field.column_name();

    let index_type = field.column().index.unwrap_or_default();
    let index_name = format!("idx_{}_{}", table, column);
    let using = index_type.as_sql_using();

    format!(
        "CREATE INDEX IF NOT EXISTS {} ON {}.{}{} ({});\n",
        index_name, schema, table, using, column
    )
}

/// Generate CREATE INDEX for a composite index.
fn generate_composite_index(entity: &EntityDef, idx: &CompositeIndexDef) -> String {
    let table = &entity.table;
    let schema = &entity.schema;

    let index_name = idx.name_or_default(table);
    let using = idx.index_type.as_sql_using();
    let unique_str = if idx.unique { "UNIQUE " } else { "" };
    let columns = idx.columns.join(", ");

    let mut sql = format!(
        "CREATE {}INDEX IF NOT EXISTS {} ON {}.{}{} ({})",
        unique_str, index_name, schema, table, using, columns
    );

    if let Some(ref where_clause) = idx.where_clause {
        sql.push_str(&format!(" WHERE {}", where_clause));
    }

    sql.push_str(";\n");
    sql
}

/// Simple pluralization for table names.
fn pluralize(s: &str) -> String {
    if s.ends_with('s') || s.ends_with("sh") || s.ends_with("ch") || s.ends_with('x') {
        format!("{}es", s)
    } else if s.ends_with('y') && !s.ends_with("ay") && !s.ends_with("ey") && !s.ends_with("oy") {
        format!("{}ies", &s[..s.len() - 1])
    } else {
        format!("{}s", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pluralize_regular() {
        assert_eq!(pluralize("user"), "users");
        assert_eq!(pluralize("post"), "posts");
    }

    #[test]
    fn pluralize_es() {
        assert_eq!(pluralize("status"), "statuses");
        assert_eq!(pluralize("match"), "matches");
    }

    #[test]
    fn pluralize_ies() {
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("company"), "companies");
    }

    #[test]
    fn pluralize_ey_oy() {
        assert_eq!(pluralize("key"), "keys");
        assert_eq!(pluralize("toy"), "toys");
    }
}
