use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Type};
use syn::{Token, parse::Parse};

/// Input for the `drizzle!` macro - simplified to only support Schema types
pub(crate) struct DrizzleInput {
    /// Connection reference
    conn: Expr,
    /// Schema type (required)
    schema_type: Type,
}

impl Parse for DrizzleInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let conn = input.parse()?;
        input.parse::<Token![,]>()?;
        let schema_type = input.parse()?;

        Ok(DrizzleInput { conn, schema_type })
    }
}

/// Implementation of the `drizzle!` macro - simplified for Schema-only support
pub fn drizzle_impl(input: DrizzleInput) -> syn::Result<TokenStream> {
    let conn = input.conn;
    let schema_type = input.schema_type;

    // Generate Drizzle instance with schema
    Ok(quote! {
        (::drizzle_rs::sqlite::Drizzle::new::<#schema_type>(#conn), #schema_type::new())
    })
}
