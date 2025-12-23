//! Entity derive macro implementation.
//!
//! This module contains all code generation logic for the `#[derive(Entity)]`
//! macro.

mod dto;
mod insertable;
mod mappers;
mod parse;
mod repository;
mod row;
mod sql;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use self::parse::EntityDef;

/// Main entry point for the Entity derive macro.
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match EntityDef::from_derive_input(&input) {
        Ok(entity) => generate(entity),
        Err(err) => err.write_errors().into()
    }
}

/// Generate all code for the entity.
fn generate(entity: EntityDef) -> TokenStream {
    let dto_tokens = dto::generate(&entity);
    let repository_tokens = repository::generate(&entity);
    let row_tokens = row::generate(&entity);
    let insertable_tokens = insertable::generate(&entity);
    let mapper_tokens = mappers::generate(&entity);
    let sql_tokens = sql::generate(&entity);

    let expanded = quote! {
        #dto_tokens
        #repository_tokens
        #row_tokens
        #insertable_tokens
        #mapper_tokens
        #sql_tokens
    };

    expanded.into()
}
