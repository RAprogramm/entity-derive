//! Field-level attribute parsing.
//!
//! This module handles parsing of field attributes like `#[id]`, `#[auto]`,
//! and `#[field(create, update, response)]`.

use syn::{Attribute, Field, Ident, Meta, Type, Visibility};

/// Field definition with all parsed attributes.
///
/// Represents a single field from the entity struct, including
/// all metadata extracted from attributes.
///
/// # Attribute Flags
///
/// | Field | Attribute | Effect |
/// |-------|-----------|--------|
/// | `is_id` | `#[id]` | Primary key, auto-generated UUID |
/// | `is_auto` | `#[auto]` | Auto-generated (timestamps) |
/// | `create` | `#[field(create)]` | Include in CreateRequest |
/// | `update` | `#[field(update)]` | Include in UpdateRequest |
/// | `response` | `#[field(response)]` | Include in Response |
/// | `skip` | `#[field(skip)]` | Exclude from all DTOs |
#[derive(Debug)]
pub struct FieldDef {
    /// Field identifier (e.g., `id`, `name`, `created_at`).
    pub ident: Ident,

    /// Field type (e.g., `Uuid`, `Option<String>`, `DateTime<Utc>`).
    pub ty: Type,

    /// Field visibility.
    ///
    /// Preserved for potential future use in generated code.
    #[allow(dead_code)]
    pub vis: Visibility,

    /// Whether this is the primary key field (`#[id]`).
    ///
    /// ID fields:
    /// - Get auto-generated UUIDs in `From<CreateRequest>`
    /// - Are always included in Response DTOs
    /// - Are excluded from CreateRequest and UpdateRequest
    pub is_id: bool,

    /// Whether this field is auto-generated (`#[auto]`).
    ///
    /// Auto fields (typically timestamps):
    /// - Get `Default::default()` in `From<CreateRequest>`
    /// - Are excluded from CreateRequest and UpdateRequest
    /// - Can still be included in Response
    pub is_auto: bool,

    /// Include in `CreateRequest` DTO.
    pub create: bool,

    /// Include in `UpdateRequest` DTO.
    pub update: bool,

    /// Include in `Response` DTO.
    pub response: bool,

    /// Exclude from all DTOs.
    ///
    /// Overrides `create`, `update`, and `response` flags.
    /// Use for internal fields like password hashes.
    pub skip: bool
}

impl FieldDef {
    /// Parse field definition from syn's `Field`.
    ///
    /// Extracts the field identifier, type, visibility, and all
    /// attribute flags.
    ///
    /// # Arguments
    ///
    /// * `field` - Parsed field from syn
    ///
    /// # Returns
    ///
    /// Populated `FieldDef` with all attributes parsed.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't have an identifier (tuple struct field).
    /// This should be caught earlier by darling's `supports(struct_named)`.
    pub fn from_field(field: &Field) -> Self {
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

    /// Get the field name as an identifier.
    ///
    /// # Returns
    ///
    /// Reference to the field's `Ident`.
    pub fn name(&self) -> &Ident {
        &self.ident
    }

    /// Get the field name as a string.
    ///
    /// Used for generating SQL column names.
    ///
    /// # Returns
    ///
    /// String representation of the field name.
    pub fn name_str(&self) -> String {
        self.ident.to_string()
    }

    /// Check if field should be in `CreateRequest`.
    ///
    /// Returns `true` if `create` is set AND `skip` is NOT set.
    pub fn in_create(&self) -> bool {
        !self.skip && self.create
    }

    /// Check if field should be in `UpdateRequest`.
    ///
    /// Returns `true` if `update` is set AND `skip` is NOT set.
    pub fn in_update(&self) -> bool {
        !self.skip && self.update
    }

    /// Check if field should be in `Response`.
    ///
    /// Returns `true` if:
    /// - `response` is set, OR `is_id` is true (IDs always in response)
    /// - AND `skip` is NOT set
    pub fn in_response(&self) -> bool {
        !self.skip && (self.response || self.is_id)
    }

    /// Check if the field type is `Option<T>`.
    ///
    /// Used to determine whether to wrap update fields in `Option`.
    /// If a field is already `Option`, it stays as-is in UpdateRequest.
    /// Non-option fields become `Option<T>` for partial updates.
    ///
    /// # Returns
    ///
    /// `true` if the type path ends with `Option`.
    ///
    /// # Limitations
    ///
    /// This is a simple heuristic that checks the last path segment.
    /// It may give false positives for custom types named `Option`.
    pub fn is_option(&self) -> bool {
        if let Type::Path(type_path) = &self.ty
            && let Some(segment) = type_path.path.segments.last()
        {
            return segment.ident == "Option";
        }
        false
    }

    /// Get the field type.
    ///
    /// # Returns
    ///
    /// Reference to the field's `Type`.
    pub fn ty(&self) -> &Type {
        &self.ty
    }
}

/// Parse `#[field(create, update, response, skip)]` attribute.
///
/// Extracts boolean flags from the nested meta list. Each identifier
/// in the list sets the corresponding flag to `true`.
///
/// # Arguments
///
/// * `attr` - The attribute to parse
/// * `create` - Mutable reference to create flag
/// * `update` - Mutable reference to update flag
/// * `response` - Mutable reference to response flag
/// * `skip` - Mutable reference to skip flag
///
/// # Recognized Identifiers
///
/// - `create` → sets `*create = true`
/// - `update` → sets `*update = true`
/// - `response` → sets `*response = true`
/// - `skip` → sets `*skip = true`
///
/// Unknown identifiers are silently ignored for forward compatibility.
///
/// # Examples
///
/// ```rust,ignore
/// #[field(create, response)]     // create=true, response=true
/// #[field(update)]               // update=true
/// #[field(skip)]                 // skip=true (overrides others)
/// #[field(create, update, response)] // all three true
/// ```
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
