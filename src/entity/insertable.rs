//! Insertable struct generation for Entity derive macro.
//!
//! Generates Insertable struct for database INSERT operations.

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EntityDef, SqlLevel};

/// Generate Insertable struct for INSERT queries.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let insertable_name = entity.ident_with("Insertable", "");
    let fields = entity.all_fields();

    let field_defs: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            let ty = f.ty();
            quote! { pub #name: #ty }
        })
        .collect();

    quote! {
        /// Insertable representation for database INSERT operations.
        #[derive(Debug, Clone)]
        #vis struct #insertable_name {
            #(#field_defs),*
        }
    }
}
