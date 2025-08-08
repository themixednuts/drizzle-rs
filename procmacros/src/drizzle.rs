use crate::schema;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, ExprArray};
use syn::{Token, parse::Parse};

/// Input for the `drizzle!` macro
pub(crate) struct DrizzleInput {
    /// Connection reference
    conn: Expr,
    /// Optional array of schema tables
    tables: Option<ExprArray>,
}

impl Parse for DrizzleInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let conn = input.parse()?;

        // Optional comma-separated schema tables
        let tables = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(DrizzleInput { conn, tables })
    }
}

/// Implementation of the `drizzle!` macro
pub fn drizzle_impl(input: DrizzleInput) -> syn::Result<TokenStream> {
    // Extract the connection and tables
    let conn = input.conn;

    // Generate output based on input tables
    let output = match &input.tables {
        Some(tables) => {
            // Extract the types from the array for schema name generation
            let types = tables
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

            // Generate the schema name
            let schema_name = schema::get_schema_name(&types);
            let schema_ident = quote::format_ident!("{}", schema_name);
            let schema_impl = schema::generate_schema(tables.to_token_stream())
                .map_err(|err| syn::Error::new(err.span(), err.to_string()))?;

            quote! {
                {
                    // Generate the schema
                    #schema_impl;

                    // // Create query builder and Drizzle instance with explicit type annotation
                    // let query_builder = ::drizzle_rs::sqlite::builder::QueryBuilder::new::<#schema_ident>();

                    (::drizzle_rs::Drizzle::new::<#schema_ident>(#conn), (#(#types::default(),)*)  )
                }
            }
        }
        None => {
            // No tables specified, use empty schema
            quote! {
                {
                    // Generate an empty schema
                    ::drizzle_rs::procmacros::schema!();

                    // let schema = ::drizzle_rs::sqlite::builder::QueryBuilder::new::<EmptySchema>();

                    ::drizzle_rs::Drizzle::new::<EmptySchema>(#conn)
                }
            }
        }
    };

    Ok(output)
}
