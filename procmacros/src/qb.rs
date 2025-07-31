use crate::schema;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Result;

/// Implementation of the `qb!` macro
pub fn qb_impl(input: proc_macro::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let input_tokens = TokenStream::from(input);
    parse_qb_input(input_tokens)
}

/// Parse the input as an optional array of table types
fn parse_qb_input(input: TokenStream) -> Result<TokenStream> {
    // Try to parse as an array
    let Ok(array) = syn::parse2::<syn::ExprArray>(input.clone()) else {
        return Err(syn::Error::new_spanned(
            input,
            "Expected an array of structs",
        ));
    };

    // Extract the types from the array to generate the schema name
    let types = array
        .elems
        .iter()
        .filter_map(|expr| {
            if let syn::Expr::Path(path) = expr {
                let type_path = syn::TypePath {
                    qself: None,
                    path: path.path.clone(),
                };
                Some(syn::Type::Path(type_path))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Generate schema name
    let schema_name = schema::get_schema_name(&types);
    let schema_ident = quote::format_ident!("{}", schema_name);
    let schema_impl = schema::generate_schema(array.to_token_stream())?;

    // Generate the QueryBuilder
    Ok(quote! {
        {
            // Define schema type
            #schema_impl;

            // Create and return a QueryBuilder
            drizzle_rs::sqlite::builder::QueryBuilder::new::<#schema_ident>()
        }
    })
}
