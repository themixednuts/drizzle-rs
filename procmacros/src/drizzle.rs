use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
    Expr, Result,
};

/// DrizzleInput holds the arguments passed to the drizzle! macro
struct DrizzleInput {
    /// The connection reference
    conn: Expr,
    /// The schema instance (optional)
    schema: Option<Expr>,
}

impl Parse for DrizzleInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let args = Punctuated::<Expr, Comma>::parse_terminated(input)?;

        if args.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "drizzle! macro requires at least a connection argument",
            ));
        }

        // Extract the arguments
        let args: Vec<_> = args.into_iter().collect();
        let conn = args[0].clone();
        let schema = if args.len() > 1 {
            Some(args[1].clone())
        } else {
            None
        };

        Ok(DrizzleInput { conn, schema })
    }
}

/// Implementation of the drizzle! macro
pub fn drizzle_macro(input: TokenStream) -> Result<TokenStream> {
    let DrizzleInput { conn, schema } = syn::parse2(input)?;

    // If schema is provided, use it, otherwise create a default Schema
    let drizzle_impl = if let Some(schema) = schema {
        quote! {
            {
                use ::drizzle_rs::connection::Drizzle;
                Drizzle::with_schema(#conn, #schema)
            }
        }
    } else {
        quote! {
            {
                use ::drizzle_rs::connection::Drizzle;
                Drizzle::new(#conn)
            }
        }
    };

    Ok(drizzle_impl)
}
