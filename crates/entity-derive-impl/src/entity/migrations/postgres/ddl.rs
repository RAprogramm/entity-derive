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
    use syn::DeriveInput;

    use super::*;
    use crate::entity::parse::EntityDef;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse_quote!(#tokens);
        EntityDef::from_derive_input(&input).unwrap()
    }

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

    #[test]
    fn pluralize_sh() {
        assert_eq!(pluralize("wish"), "wishes");
        assert_eq!(pluralize("bush"), "bushes");
    }

    #[test]
    fn pluralize_x() {
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("fox"), "foxes");
    }

    #[test]
    fn pluralize_ay() {
        assert_eq!(pluralize("day"), "days");
        assert_eq!(pluralize("way"), "ways");
    }

    #[test]
    fn generate_up_basic() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS public.users"));
        assert!(sql.contains("id UUID PRIMARY KEY"));
        assert!(sql.contains("name TEXT NOT NULL"));
    }

    #[test]
    fn generate_down_basic() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", schema = "core", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        let sql = generate_down(&entity);
        assert_eq!(sql, "DROP TABLE IF EXISTS core.users CASCADE;\n");
    }

    #[test]
    fn generate_up_with_unique() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
    }

    #[test]
    fn generate_up_with_default() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(default = "true")]
                pub active: bool,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("DEFAULT true"));
    }

    #[test]
    fn generate_up_with_check() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(check = "age >= 0")]
                pub age: i32,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("CHECK (age >= 0)"));
    }

    #[test]
    fn generate_up_with_index() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(index)]
                pub status: String,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("CREATE INDEX IF NOT EXISTS idx_users_status"));
    }

    #[test]
    fn generate_up_with_gin_index() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(index = "gin")]
                pub tags: Vec<String>,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("USING gin"));
    }

    #[test]
    fn generate_up_with_nullable() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub bio: Option<String>,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("bio TEXT"));
        assert!(!sql.contains("bio TEXT NOT NULL"));
    }

    #[test]
    fn generate_up_with_varchar() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(varchar = 100)]
                pub name: String,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("VARCHAR(100)"));
    }

    #[test]
    fn generate_up_with_belongs_to() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "posts", migrations)]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[belongs_to(User)]
                pub user_id: uuid::Uuid,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("REFERENCES public.users(id)"));
    }

    #[test]
    fn generate_up_with_belongs_to_on_delete_cascade() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "posts", migrations)]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[belongs_to(User, on_delete = "cascade")]
                pub user_id: uuid::Uuid,
            }
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("REFERENCES public.users(id) ON DELETE CASCADE"));
    }

    #[test]
    fn generate_composite_index_basic() {
        let idx = CompositeIndexDef {
            name:         None,
            columns:      vec!["name".to_string(), "email".to_string()],
            index_type:   crate::entity::parse::IndexType::BTree,
            unique:       false,
            where_clause: None
        };
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
                #[field(create, response)]
                pub email: String,
            }
        });
        let sql = generate_composite_index(&entity, &idx);
        assert!(sql.contains("CREATE INDEX IF NOT EXISTS idx_users_name_email"));
        assert!(sql.contains("(name, email)"));
    }

    #[test]
    fn generate_composite_index_unique() {
        let idx = CompositeIndexDef {
            name:         None,
            columns:      vec!["tenant_id".to_string(), "email".to_string()],
            index_type:   crate::entity::parse::IndexType::BTree,
            unique:       true,
            where_clause: None
        };
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        let sql = generate_composite_index(&entity, &idx);
        assert!(sql.contains("CREATE UNIQUE INDEX"));
        assert!(sql.contains("(tenant_id, email)"));
    }

    #[test]
    fn generate_composite_index_with_where() {
        let idx = CompositeIndexDef {
            name:         Some("idx_active_users".to_string()),
            columns:      vec!["email".to_string()],
            index_type:   crate::entity::parse::IndexType::BTree,
            unique:       false,
            where_clause: Some("active = true".to_string())
        };
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        let sql = generate_composite_index(&entity, &idx);
        assert!(sql.contains("idx_active_users"));
        assert!(sql.contains("WHERE active = true"));
    }

    #[test]
    fn generate_composite_index_gin() {
        let idx = CompositeIndexDef {
            name:         None,
            columns:      vec!["tags".to_string()],
            index_type:   crate::entity::parse::IndexType::Gin,
            unique:       false,
            where_clause: None
        };
        let entity = parse_entity(quote::quote! {
            #[entity(table = "posts", migrations)]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        let sql = generate_composite_index(&entity, &idx);
        assert!(sql.contains("USING gin"));
    }

    #[test]
    fn generate_up_with_composite_indexes() {
        let mut entity = parse_entity(quote::quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
                #[field(create, response)]
                pub email: String,
            }
        });
        entity.indexes.push(CompositeIndexDef {
            name:         None,
            columns:      vec!["name".to_string(), "email".to_string()],
            index_type:   crate::entity::parse::IndexType::BTree,
            unique:       false,
            where_clause: None
        });
        let sql = generate_up(&entity);
        assert!(sql.contains("CREATE INDEX IF NOT EXISTS idx_users_name_email"));
    }
}
