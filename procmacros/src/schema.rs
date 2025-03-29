use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Result, Token, Type, parse2, token};

// Struct to parse the input like `[Type1, Type2]`
struct SchemaInput {
    bracket_token: token::Bracket,
    tables: Punctuated<Type, Token![,]>, // Corrected: Use Type, not Ident
}

impl Parse for SchemaInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            bracket_token: syn::bracketed!(content in input),
            tables: content.parse_terminated(Type::parse, Token![,])?,
        })
    }
}

pub fn schema_macro_impl(input: TokenStream) -> Result<TokenStream> {
    let parsed_input = parse2::<SchemaInput>(input)?;

    // Generate a unique (ish) module name and marker type name
    // Using a fixed name for simplicity now. Could use span ID later if needed.
    let module_name = quote! { __drizzle_schema_marker_module };
    let marker_name = quote! { SchemaMarker };

    let tables = parsed_input.tables.iter();

    // Generate `impl IsInSchema<Marker> for Table {}` for each table
    let impls = tables.map(|table_type| {
        // Use ::drizzle_rs path for IsInSchema
        quote! {
            impl<'a> ::drizzle_rs::prelude::IsInSchema<#module_name::#marker_name> for #table_type<'a> {}
        }
    });

    // Combine the generated code
    let expanded = quote! {
        {
            // Define the marker module and type
            mod #module_name {
                #[derive(Clone)]
                pub struct #marker_name;
            }

            // Implement the trait for each table
            #(#impls)*

            // Return the query builder factory instance parameterized with the marker type
            // Use the ::querybuilder path to the schema function
            ::drizzle_rs::sqlite::query_builder::schema::<#module_name::#marker_name>()
        }
    };

    Ok(expanded)
}
