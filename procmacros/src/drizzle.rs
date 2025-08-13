use crate::schema;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, ExprArray};
use syn::{Token, parse::Parse};

/// Input for the `drizzle!` macro
pub(crate) struct DrizzleInput {
    /// Connection reference
    conn: Expr,
    /// Optional schema tables - either a single table or an array of tables
    tables: Option<TableInput>,
}

/// Represents the table input - either a single table or an array
pub(crate) enum TableInput {
    /// Single table: drizzle!(conn, Table)
    Single(Expr),
    /// Array of tables: drizzle!(conn, [Table1, Table2])
    Array(ExprArray),
}

impl Parse for DrizzleInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let conn = input.parse()?;

        // Optional comma-separated schema tables
        let tables = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            
            // Check if it's an array [Table1, Table2] or a single Table
            if input.peek(syn::token::Bracket) {
                // Array syntax: [Table1, Table2]
                Some(TableInput::Array(input.parse()?))
            } else {
                // Single table syntax: Table
                Some(TableInput::Single(input.parse()?))
            }
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
        Some(table_input) => {
            match table_input {
                TableInput::Single(single_table) => {
                    // Handle single table: drizzle!(conn, Table) -> same as drizzle!(conn, [Table])
                    
                    // Extract the type from the single table expression
                    let table_type = if let syn::Expr::Path(path) = single_table {
                        let type_path = syn::TypePath {
                            qself: None,
                            path: path.path.clone(),
                        };
                        syn::Type::Path(type_path)
                    } else {
                        return Err(syn::Error::new_spanned(single_table, "Expected a table type"));
                    };

                    let types = vec![table_type];
                    
                    // Create a fake array with the single element for schema generation
                    let fake_array = quote! { [#single_table] };
                    
                    // Generate the schema name
                    let schema_name = schema::get_schema_name(&types);
                    let schema_ident = quote::format_ident!("{}", schema_name);
                    let schema_impl = schema::generate_schema(fake_array)
                        .map_err(|err| syn::Error::new(err.span(), err.to_string()))?;

                    // Single table always returns the table directly (not in a tuple)
                    quote! {
                        {
                            #schema_impl;
                            (::drizzle_rs::sqlite::Drizzle::new::<#schema_ident>(#conn) , #(#types::default(),)*  )
                        }
                    }
                }
                TableInput::Array(tables) => {
                    // Handle array syntax: drizzle!(conn, [Table1, Table2])
                    
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

                    if types.len() == 1 {
                        quote! {
                            {
                                #schema_impl;
                                (::drizzle_rs::sqlite::Drizzle::new::<#schema_ident>(#conn) , #(#types::default(),)*  )
                            }
                        }
                    } else {
                        quote! {
                            {
                                #schema_impl;
                                (::drizzle_rs::sqlite::Drizzle::new::<#schema_ident>(#conn) , (#(#types::default(),)*)  )
                            }
                        }
                    }
                }
            }
        }
        None => {
            // No tables specified, use empty schema
            quote! {
                {
                    ::drizzle_rs::Drizzle::new::<EmptySchema>(#conn)
                }
            }
        }
    };

    Ok(output)
}
