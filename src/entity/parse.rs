//! Attribute parsing for Entity derive macro.
//!
//! Uses `darling` for entity-level attributes and manual parsing for field
//! attributes.

use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromMeta};
use proc_macro2::Span;
use syn::{Attribute, DeriveInput, Field, Ident, Meta, Type, Visibility};

/// SQL generation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SqlLevel {
    /// Generate trait + impl for PgPool
    #[default]
    Full,
    /// Generate only trait, SQL manually
    Trait,
    /// No repository generation
    None
}

impl FromMeta for SqlLevel {
    fn from_string(value: &str) -> darling::Result<Self> {
        match value.to_lowercase().as_str() {
            "full" => Ok(SqlLevel::Full),
            "trait" => Ok(SqlLevel::Trait),
            "none" => Ok(SqlLevel::None),
            _ => Err(darling::Error::unknown_value(value))
        }
    }
}

/// Entity-level attributes from `#[entity(...)]`.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(entity), supports(struct_named))]
struct EntityAttrs {
    /// Struct identifier
    ident: Ident,

    /// Struct visibility
    vis: Visibility,

    /// Database table name (required)
    table: String,

    /// Database schema (default: "public")
    #[darling(default = "default_schema")]
    schema: String,

    /// SQL generation level
    #[darling(default)]
    sql: SqlLevel
}

fn default_schema() -> String {
    "public".to_string()
}

/// Parsed entity definition.
#[derive(Debug)]
pub struct EntityDef {
    /// Struct identifier
    pub ident: Ident,

    /// Struct visibility
    pub vis: Visibility,

    /// Database table name
    pub table: String,

    /// Database schema
    pub schema: String,

    /// SQL generation level
    pub sql: SqlLevel,

    /// Parsed fields
    pub fields: Vec<FieldDef>
}

impl EntityDef {
    /// Parse from syn DeriveInput.
    pub fn from_derive_input(input: &DeriveInput) -> darling::Result<Self> {
        let attrs = EntityAttrs::from_derive_input(input)?;

        let fields = match &input.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(named) => {
                    named.named.iter().map(FieldDef::from_field).collect()
                }
                _ => {
                    return Err(darling::Error::custom("Entity requires named fields")
                        .with_span(&input.ident));
                }
            },
            _ => {
                return Err(
                    darling::Error::custom("Entity can only be derived for structs")
                        .with_span(&input.ident)
                );
            }
        };

        Ok(Self {
            ident: attrs.ident,
            vis: attrs.vis,
            table: attrs.table,
            schema: attrs.schema,
            sql: attrs.sql,
            fields
        })
    }

    /// Get the primary key field.
    pub fn id_field(&self) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.is_id)
    }

    /// Get fields for CreateRequest.
    pub fn create_fields(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| f.in_create() && !f.is_id && !f.is_auto)
            .collect()
    }

    /// Get fields for UpdateRequest.
    pub fn update_fields(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| f.in_update() && !f.is_id && !f.is_auto)
            .collect()
    }

    /// Get fields for Response.
    pub fn response_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.in_response()).collect()
    }

    /// Get all fields (for Row/Insertable).
    pub fn all_fields(&self) -> &[FieldDef] {
        &self.fields
    }

    /// Entity name (e.g., "User").
    pub fn name(&self) -> &Ident {
        &self.ident
    }

    /// Entity name as string.
    pub fn name_str(&self) -> String {
        self.ident.to_string()
    }

    /// Snake case name (e.g., "user").
    pub fn snake_name(&self) -> String {
        self.name_str().to_case(Case::Snake)
    }

    /// Full table name with schema (e.g., "core.users").
    pub fn full_table_name(&self) -> String {
        format!("{}.{}", self.schema, self.table)
    }

    /// Create ident with prefix/suffix.
    pub fn ident_with(&self, prefix: &str, suffix: &str) -> Ident {
        Ident::new(
            &format!("{}{}{}", prefix, self.name_str(), suffix),
            Span::call_site()
        )
    }
}

/// Field definition with all attributes.
#[derive(Debug)]
pub struct FieldDef {
    /// Field identifier
    pub ident: Ident,

    /// Field type
    pub ty: Type,

    /// Field visibility
    pub vis: Visibility,

    /// Is this the primary key? (`#[id]`)
    pub is_id: bool,

    /// Is this auto-generated? (`#[auto]`)
    pub is_auto: bool,

    /// Include in CreateRequest
    pub create: bool,

    /// Include in UpdateRequest
    pub update: bool,

    /// Include in Response
    pub response: bool,

    /// Skip this field in all DTOs
    pub skip: bool
}

impl FieldDef {
    /// Parse from syn Field.
    fn from_field(field: &Field) -> Self {
        let ident = field.ident.clone().expect("named field required");
        let ty = field.ty.clone();
        let vis = field.vis.clone();

        let mut is_id = false;
        let mut is_auto = false;
        let mut create = false;
        let mut update = false;
        let mut response = false;
        let mut skip = false;

        for attr in &field.attrs {
            if attr.path().is_ident("id") {
                is_id = true;
            } else if attr.path().is_ident("auto") {
                is_auto = true;
            } else if attr.path().is_ident("field") {
                parse_field_attr(attr, &mut create, &mut update, &mut response, &mut skip);
            }
        }

        Self {
            ident,
            ty,
            vis,
            is_id,
            is_auto,
            create,
            update,
            response,
            skip
        }
    }

    /// Get field name as Ident.
    pub fn name(&self) -> &Ident {
        &self.ident
    }

    /// Get field name as string.
    pub fn name_str(&self) -> String {
        self.ident.to_string()
    }

    /// Check if field should be in CreateRequest.
    pub fn in_create(&self) -> bool {
        !self.skip && self.create
    }

    /// Check if field should be in UpdateRequest.
    pub fn in_update(&self) -> bool {
        !self.skip && self.update
    }

    /// Check if field should be in Response.
    pub fn in_response(&self) -> bool {
        !self.skip && (self.response || self.is_id)
    }

    /// Check if field type is Option<T>.
    pub fn is_option(&self) -> bool {
        if let Type::Path(type_path) = &self.ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Option";
            }
        }
        false
    }

    /// Get the field type.
    pub fn ty(&self) -> &Type {
        &self.ty
    }
}

/// Parse `#[field(create, update, response, skip)]` attribute.
fn parse_field_attr(
    attr: &Attribute,
    create: &mut bool,
    update: &mut bool,
    response: &mut bool,
    skip: &mut bool
) {
    if let Meta::List(meta_list) = &attr.meta {
        let _ = meta_list.parse_nested_meta(|meta| {
            if meta.path.is_ident("create") {
                *create = true;
            } else if meta.path.is_ident("update") {
                *update = true;
            } else if meta.path.is_ident("response") {
                *response = true;
            } else if meta.path.is_ident("skip") {
                *skip = true;
            }
            Ok(())
        });
    }
}
