//! Entity derive macro implementation.
//!
//! Contains all code generation logic for `#[derive(Entity)]`.

mod dto;
mod insertable;
mod mappers;
pub mod parse;
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

fn generate(entity: EntityDef) -> TokenStream {
    let dto = dto::generate(&entity);
    let repository = repository::generate(&entity);
    let row = row::generate(&entity);
    let insertable = insertable::generate(&entity);
    let mappers = mappers::generate(&entity);
    let sql = sql::generate(&entity);

    let expanded = quote! {
        #dto
        #repository
        #row
        #insertable
        #mappers
        #sql
    };

    expanded.into()
}
