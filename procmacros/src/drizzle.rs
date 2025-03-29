use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, ExprArray, Path, Result, Type, TypeArray, TypePath,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
};

/// DrizzleInput holds the arguments passed to the drizzle! macro
struct DrizzleInput {
    /// The connection reference
    conn: Expr,
    /// The schema tables (array of types)
    tables: Option<ExprArray>,
}

impl Parse for DrizzleInput {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse the connection argument
        let conn = input.parse()?;

        // Check if there's a comma and potentially tables
        if input.peek(Comma) {
            // Consume the comma
            input.parse::<Comma>()?;

            // Expect the table array
            let tables_expr: ExprArray = input.parse()?;
            Ok(DrizzleInput {
                conn,
                tables: Some(tables_expr),
            })
        } else {
            // No comma, so no tables provided
            Ok(DrizzleInput { conn, tables: None })
        }
    }
}

/// Implementation of the drizzle! macro
pub fn drizzle_macro(input: TokenStream) -> Result<TokenStream> {
    let DrizzleInput { conn, tables } = syn::parse2(input)?;

    // Handle the table array
    let drizzle_impl = if let Some(tables) = tables {
        // Extract table names from the array
        let elems = &tables.elems;

        quote! {
            {
                use ::drizzle_rs::Drizzle;

                // Create a query builder with the schema info
                let query_builder = schema!([#elems]);

                // Create a Drizzle instance with schema
                Drizzle::with_schema(#conn, query_builder)
            }
        }
    } else {
        // No tables provided - just create the base Drizzle without schema
        quote! {
            {
                use ::drizzle_rs::Drizzle;
                Drizzle::new(#conn)
            }
        }
    };

    Ok(drizzle_impl)
}
