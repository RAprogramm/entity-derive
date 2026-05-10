// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Repository trait generation.
//!
//! Generates an async repository trait with standard CRUD operations.
//! The trait serves as a database abstraction layer, allowing different
//! backend implementations (PostgreSQL, ClickHouse, MongoDB).
//!
//! # Generated Trait
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! #[async_trait]
//! pub trait UserRepository: Send + Sync {
//!     type Error: std::error::Error + Send + Sync;
//!     type Pool;
//!
//!     fn pool(&self) -> &Self::Pool;
//!     async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error>;
//!     async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
//!     async fn update(&self, id: Uuid, dto: UpdateUserRequest) -> Result<User, Self::Error>;
//!     async fn delete(&self, id: Uuid) -> Result<bool, Self::Error>;
//!     async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, Self::Error>;
//! }
//! ```
//!
//! # Associated Types
//!
//! - `Error` — custom error type (default: `sqlx::Error`)
//! - `Pool` — database pool type for transaction support
//!
//! # Conditional Generation
//!
//! Methods are generated based on entity configuration:
//!
//! | Method | Condition |
//! |--------|-----------|
//! | `create` | Entity has `#[field(create)]` fields |
//! | `update` | Entity has `#[field(update)]` fields |
//! | `find_by_id`, `delete`, `list` | Always generated |
//!
//! # SQL Level Control
//!
//! - `sql = "full"` — generates trait + implementation
//! - `sql = "trait"` — generates trait only (implement manually)
//! - `sql = "none"` — no repository generation

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_quote;

use super::parse::{EntityDef, FieldDef, SqlLevel};
use crate::utils::marker;

/// Lookup method definition with all generated code.
struct LookupMethodDef {
    /// Method name (e.g., `find_by_email`).
    name:        syn::Ident,
    /// Parameter name (e.g., `email`).
    param_name:  syn::Ident,
    /// Parameter type (e.g., `String`).
    param_type:  syn::Type,
    /// Return type (e.g., `Option<User>` or `bool`).
    return_type: syn::Type,
    /// Whether this method should be generated.
    generated:   bool
}

/// Generates the repository trait definition.
///
/// Returns an empty `TokenStream` if `sql = "none"` is specified.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let trait_name = format_ident!("{}Repository", entity_name);
    let create_dto = entity.ident_with("Create", "Request");
    let update_dto = entity.ident_with("Update", "Request");

    let id_type = entity.id_field().ty();

    let create_method = if entity.create_fields().is_empty() {
        TokenStream::new()
    } else {
        quote! { async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error>; }
    };

    let update_method = if entity.update_fields().is_empty() {
        TokenStream::new()
    } else {
        quote! { async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error>; }
    };

    let relation_methods = generate_relation_methods(entity, id_type);
    let projection_methods = generate_projection_methods(entity, id_type);
    let soft_delete_methods = generate_soft_delete_methods(entity, id_type);
    let query_method = generate_query_method(entity);
    let stream_method = generate_stream_method(entity);
    let lookup_methods = generate_lookup_methods(entity, id_type);
    let save_method = generate_save_method(entity);
    let marker = marker::generated();

    quote! {
        #marker
        #[async_trait::async_trait]
        #vis trait #trait_name: Send + Sync {
            /// Error type for repository operations.
            type Error: std::error::Error + Send + Sync;

            /// Underlying database pool type.
            type Pool;

            /// Get reference to the underlying database pool.
            ///
            /// Enables transactions and custom queries:
            /// ```ignore
            /// let pool = repo.pool();
            /// let mut tx = pool.begin().await?;
            /// // ... custom operations
            /// tx.commit().await?;
            /// ```
            fn pool(&self) -> &Self::Pool;

            #create_method

            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error>;

            #update_method

            async fn delete(&self, id: #id_type) -> Result<bool, Self::Error>;

            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error>;

            #query_method

            #stream_method

            #lookup_methods

            #relation_methods

            #projection_methods

            #soft_delete_methods

            #save_method
        }
    }
}

/// Generate relation methods for `#[belongs_to]` and `#[has_many]`.
///
/// For `#[belongs_to(Entity)]`, generates:
/// ```rust,ignore
/// async fn find_{entity_snake}(&self, id: IdType) -> Result<Option<Entity>, Self::Error>;
/// ```
///
/// For `#[has_many(Entity)]`, generates:
/// ```rust,ignore
/// async fn find_{entity_snake_plural}(&self, id: IdType) -> Result<Vec<Entity>, Self::Error>;
/// ```
fn generate_relation_methods(entity: &EntityDef, id_type: &syn::Type) -> TokenStream {
    let belongs_to_methods: Vec<TokenStream> = entity
        .relation_fields()
        .iter()
        .filter_map(|field| generate_belongs_to_method(field, id_type))
        .collect();

    let has_many_methods: Vec<TokenStream> = entity
        .has_many_relations()
        .iter()
        .map(|related| generate_has_many_method(entity, related, id_type))
        .collect();

    quote! {
        #(#belongs_to_methods)*
        #(#has_many_methods)*
    }
}

/// Generate a single `find_{entity}` method for a belongs_to relation.
fn generate_belongs_to_method(field: &FieldDef, id_type: &syn::Type) -> Option<TokenStream> {
    let related_entity = field.belongs_to()?;
    let method_name = format_ident!("find_{}", related_entity.to_string().to_case(Case::Snake));

    Some(quote! {
        /// Find the related entity for this foreign key.
        async fn #method_name(&self, id: #id_type) -> Result<Option<#related_entity>, Self::Error>;
    })
}

/// Generate a `find_{entities}` method for a has_many relation.
fn generate_has_many_method(
    entity: &EntityDef,
    related: &syn::Ident,
    id_type: &syn::Type
) -> TokenStream {
    let related_snake = related.to_string().to_case(Case::Snake);
    let method_name = format_ident!("find_{}s", related_snake);
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let fk_field = format_ident!("{}_id", entity_snake);

    quote! {
        /// Find all related entities for this parent.
        ///
        /// The foreign key field is assumed to be `{parent}_id`.
        async fn #method_name(&self, #fk_field: #id_type) -> Result<Vec<#related>, Self::Error>;
    }
}

/// Generate projection methods for `#[projection(Name: fields)]`.
///
/// For each projection, generates:
/// ```rust,ignore
/// async fn find_by_id_public(&self, id: Uuid) -> Result<Option<UserPublic>, Self::Error>;
/// ```
fn generate_projection_methods(entity: &EntityDef, id_type: &syn::Type) -> TokenStream {
    let entity_name = entity.name();

    let methods: Vec<TokenStream> = entity
        .projections
        .iter()
        .map(|proj| {
            let proj_snake = proj.name.to_string().to_case(Case::Snake);
            let method_name = format_ident!("find_by_id_{}", proj_snake);
            let proj_type = format_ident!("{}{}", entity_name, proj.name);

            quote! {
                /// Find entity by ID as projection (optimized SELECT).
                async fn #method_name(&self, id: #id_type) -> Result<Option<#proj_type>, Self::Error>;
            }
        })
        .collect();

    quote! { #(#methods)* }
}

/// Generate soft delete methods when `#[entity(soft_delete)]` is enabled.
///
/// Generates:
/// - `hard_delete` — actual DELETE from database
/// - `restore` — set `deleted_at = NULL` to undelete
/// - `find_by_id_with_deleted` — find without filtering deleted records
/// - `list_with_deleted` — list without filtering deleted records
fn generate_soft_delete_methods(entity: &EntityDef, id_type: &syn::Type) -> TokenStream {
    if !entity.is_soft_delete() {
        return TokenStream::new();
    }

    let entity_name = entity.name();

    quote! {
        /// Permanently remove entity from database.
        ///
        /// Unlike `delete`, this actually removes the row.
        async fn hard_delete(&self, id: #id_type) -> Result<bool, Self::Error>;

        /// Restore a soft-deleted entity.
        ///
        /// Sets `deleted_at = NULL` to make the entity visible again.
        async fn restore(&self, id: #id_type) -> Result<bool, Self::Error>;

        /// Find entity by ID including soft-deleted records.
        ///
        /// Unlike `find_by_id`, this does not filter out deleted records.
        async fn find_by_id_with_deleted(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error>;

        /// List entities including soft-deleted records.
        ///
        /// Unlike `list`, this does not filter out deleted records.
        async fn list_with_deleted(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error>;
    }
}

/// Generate query method when entity has filter fields.
///
/// Generates:
/// ```rust,ignore
/// async fn query(&self, query: UserQuery) -> Result<Vec<User>, Self::Error>;
/// ```
fn generate_query_method(entity: &EntityDef) -> TokenStream {
    if !entity.has_filters() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let query_type = entity.ident_with("", "Query");

    quote! {
        /// Query entities with type-safe filters.
        ///
        /// Supports filtering by fields marked with `#[filter]`.
        async fn query(&self, query: #query_type) -> Result<Vec<#entity_name>, Self::Error>;
    }
}

/// Generate stream method when entity has streams feature and filters.
///
/// Generates:
/// ```rust,ignore
/// async fn stream_filtered(
///     &self,
///     filter: UserFilter,
/// ) -> Result<impl futures::Stream<Item = Result<User, sqlx::Error>>, Self::Error>;
/// ```
pub fn generate_stream_method(entity: &EntityDef) -> TokenStream {
    if !entity.has_streams() || !entity.has_filters() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let filter_type = entity.ident_with("", "Filter");

    quote! {
        /// Stream entities with type-safe filters.
        ///
        /// Returns an async stream for memory-efficient processing of large result sets.
        async fn stream_filtered(
            &self,
            filter: #filter_type,
        ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<#entity_name, Self::Error>> + Send + '_>>, Self::Error>;
    }
}

/// Generate lookup methods for fields with `#[column(unique)]` or
/// `#[column(index)]`.
///
/// For fields with `unique`: generates `find_by_{field}` + `exists_by_{field}`
/// For fields with `index` only: generates `find_by_{field}`
///
/// # Trait Methods Generated
///
/// | Field config | Methods generated |
/// |--------------|-------------------|
/// | `#[column(unique)]` | `find_by_{field}`, `exists_by_{field}` |
/// | `#[column(index)]` | `find_by_{field}` |
/// | `#[column(unique, index)]` | `find_by_{field}`, `exists_by_{field}` |
///
/// # Parameter Types
///
/// Parameters use owned types (e.g., `email: String` not `email: &String`)
/// to match the existing `find_by_id(&self, id: Uuid)` pattern.
pub fn generate_lookup_methods(entity: &EntityDef, id_type: &syn::Type) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let methods: Vec<TokenStream> = entity
        .lookup_fields()
        .iter()
        .flat_map(|field| generate_lookup_method_defs(field, entity.name(), id_type))
        .filter(|d| d.generated)
        .map(|d| generate_trait_method(&d))
        .collect();

    quote! { #(#methods)* }
}

/// Generate lookup method definitions for a single field.
///
/// Returns a vector of `LookupMethodDef` — one for `find_by` and optionally
/// one for `exists_by` (unique fields only).
fn generate_lookup_method_defs(
    field: &FieldDef,
    entity_name: &syn::Ident,
    _id_type: &syn::Type
) -> Vec<LookupMethodDef> {
    let field_name = field.name();
    let field_name_str = field.name_str();
    let field_type = field.ty();
    let find_name = format_ident!("find_by_{}", field_name_str);
    let exists_name = format_ident!("exists_by_{}", field_name_str);

    let option_return_type: syn::Type = parse_quote!(Option<#entity_name>);

    let mut methods = Vec::new();

    // find_by is always generated for unique or index fields
    methods.push(LookupMethodDef {
        name:        find_name,
        param_name:  field_name.clone(),
        param_type:  field_type.clone(),
        return_type: option_return_type,
        generated:   true
    });

    // exists_by is only for unique fields
    if field.column.unique {
        methods.push(LookupMethodDef {
            name:        exists_name,
            param_name:  field_name.clone(),
            param_type:  field_type.clone(),
            return_type: syn::Type::Path(syn::TypePath {
                qself: None,
                path:  syn::Path::from(format_ident!("bool"))
            }),
            generated:   true
        });
    }

    methods
}

/// Generate a single trait method token stream from a lookup method definition.
fn generate_trait_method(def: &LookupMethodDef) -> TokenStream {
    let method_name = &def.name;
    let param_name = &def.param_name;
    let param_type = &def.param_type;
    let return_type = &def.return_type;

    let doc_comment = if def.name.to_string().starts_with("exists_by_") {
        quote! {
            /// Check if a record exists with the given field value.
            ///
            /// Uses `SELECT EXISTS(SELECT 1 FROM ... WHERE field = $1)` for
            /// efficient existence check.
        }
    } else {
        quote! {
            /// Find a single entity by the given field value.
            ///
            /// Returns `None` if no record matches. For unique fields, at most
            /// one record will match. For indexed fields, returns the first match.
        }
    };

    quote! {
        #doc_comment
        async fn #method_name(&self, #param_name: #param_type) -> Result<#return_type, Self::Error>;
    }
}

/// Generate `save` method for aggregate root entities.
///
/// When `aggregate_root` is enabled, replaces `create` with `save` that takes
/// `New{Entity}` and performs a transactional insert of the parent and all
/// children.
fn generate_save_method(entity: &EntityDef) -> TokenStream {
    if !entity.is_aggregate_root() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let new_name = entity.ident_with("New", "");

    quote! {
        /// Save an aggregate root with all its children in a single transaction.
        ///
        /// This method inserts the parent entity and all child entities
        /// atomically. If any child insert fails, the entire operation
        /// is rolled back.
        async fn save(&self, new: #new_name) -> Result<#entity_name, Self::Error>;
    }
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    use super::*;
    use crate::entity::parse::EntityDef;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = parse_quote!(#tokens);
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn lookup_fields_returns_unique_and_index() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
                #[field(create, response)]
                #[column(index)]
                pub status: String,
                #[field(create, response)]
                pub name: String,
            }
        });

        let fields = entity.lookup_fields();
        assert_eq!(fields.len(), 2);

        let names: Vec<String> = fields.iter().map(|f| f.name_str()).collect();
        assert!(names.contains(&"email".to_string()));
        assert!(names.contains(&"status".to_string()));
        assert!(!names.contains(&"name".to_string()));
    }

    #[test]
    fn lookup_fields_unique_only() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
                #[field(create, response)]
                pub name: String,
            }
        });

        let fields = entity.lookup_fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name_str(), "email");
    }

    #[test]
    fn lookup_fields_index_only() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "posts")]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(index)]
                pub slug: String,
                #[field(create, response)]
                pub title: String,
            }
        });

        let fields = entity.lookup_fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name_str(), "slug");
    }

    #[test]
    fn lookup_fields_none() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });

        let fields = entity.lookup_fields();
        assert!(fields.is_empty());
    }

    #[test]
    fn lookup_fields_both_unique_and_index() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "products")]
            pub struct Product {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique, index)]
                pub sku: String,
                #[field(create, response)]
                pub name: String,
            }
        });

        let fields = entity.lookup_fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name_str(), "sku");
    }

    #[test]
    fn generate_lookup_methods_unique_generates_both() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let methods = generate_lookup_methods(&entity, &id_type);
        let code = methods.to_string();

        assert!(code.contains("find_by_email"));
        assert!(code.contains("exists_by_email"));
    }

    #[test]
    fn generate_lookup_methods_index_only_generates_find() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "posts")]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(index)]
                pub slug: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let methods = generate_lookup_methods(&entity, &id_type);
        let code = methods.to_string();

        assert!(code.contains("find_by_slug"));
        assert!(!code.contains("exists_by_slug"));
    }

    #[test]
    fn generate_lookup_methods_none_returns_empty() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", sql = "none")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let methods = generate_lookup_methods(&entity, &id_type);
        assert!(methods.is_empty());
    }

    #[test]
    fn generate_lookup_methods_trait_generates_methods() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", sql = "trait")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let methods = generate_lookup_methods(&entity, &id_type);
        let code = methods.to_string();

        assert!(code.contains("find_by_email"));
        assert!(code.contains("exists_by_email"));
    }

    #[test]
    fn generate_lookup_methods_multiple_fields() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "products")]
            pub struct Product {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique, index)]
                pub sku: String,
                #[field(create, response)]
                #[column(index)]
                pub status: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let methods = generate_lookup_methods(&entity, &id_type);
        let code = methods.to_string();

        assert!(code.contains("find_by_sku"));
        assert!(code.contains("exists_by_sku"));
        assert!(code.contains("find_by_status"));
        assert!(!code.contains("exists_by_status"));
    }

    #[test]
    fn lookup_method_defs_unique_field() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let fields = entity.lookup_fields();
        assert_eq!(fields.len(), 1);

        let defs = generate_lookup_method_defs(fields[0], entity.name(), &id_type);
        assert_eq!(defs.len(), 2);

        assert_eq!(defs[0].name.to_string(), "find_by_email");
        assert_eq!(defs[1].name.to_string(), "exists_by_email");
    }

    #[test]
    fn lookup_method_defs_index_field() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "posts")]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(index)]
                pub slug: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let fields = entity.lookup_fields();
        assert_eq!(fields.len(), 1);

        let defs = generate_lookup_method_defs(fields[0], entity.name(), &id_type);
        assert_eq!(defs.len(), 1);

        assert_eq!(defs[0].name.to_string(), "find_by_slug");
    }

    #[test]
    fn lookup_method_defs_trait_method_contains_async() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let fields = entity.lookup_fields();
        let defs = generate_lookup_method_defs(fields[0], entity.name(), &id_type);
        let method = generate_trait_method(&defs[0]);
        let code = method.to_string();

        assert!(code.contains("async fn"));
        assert!(code.contains("find_by_email"));
        assert!(code.contains("Result"));
    }

    #[test]
    fn lookup_method_exists_trait_method() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let id_type: syn::Type = parse_quote!(uuid::Uuid);
        let fields = entity.lookup_fields();
        let defs = generate_lookup_method_defs(fields[0], entity.name(), &id_type);
        assert_eq!(defs.len(), 2);

        let method = generate_trait_method(&defs[1]);
        let code = method.to_string();

        assert!(code.contains("async fn"));
        assert!(code.contains("exists_by_email"));
        assert!(code.contains("Result"));
        assert!(code.contains("bool"));
    }
}
