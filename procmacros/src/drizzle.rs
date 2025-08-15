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
    /// User-provided schema type
    schema_type: Option<Expr>,
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

        // Optional schema type as last argument
        let schema_type = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(DrizzleInput { conn, tables, schema_type })
    }
}

/// Implementation of the `drizzle!` macro
pub fn drizzle_impl(input: DrizzleInput) -> syn::Result<TokenStream> {
    // Extract the connection, tables, and schema type
    let conn = input.conn;
    let schema_type = input.schema_type;

    // Generate output based on input tables and schema type
    let output = if let Some(schema_expr) = schema_type {
        // User provided a schema type
        match &input.tables {
            Some(table_input) => {
                let (types, tables_tokens) = match table_input {
                    TableInput::Single(single_table) => {
                        // Extract the type from the single table expression
                        let table_type = if let syn::Expr::Path(path) = single_table {
                            let type_path = syn::TypePath {
                                qself: None,
                                path: path.path.clone(),
                            };
                            syn::Type::Path(type_path)
                        } else {
                            return Err(syn::Error::new_spanned(
                                single_table,
                                "Expected a table type",
                            ));
                        };
                        
                        let types = vec![table_type];
                        let fake_array = quote! { [#single_table] };
                        (types, fake_array)
                    }
                    TableInput::Array(tables) => {
                        // Extract the types from the array
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
                        (types, tables.to_token_stream())
                    }
                };

                // Generate schema implementation with user-provided type
                let schema_impl = schema::generate_schema_for_type(&schema_expr, tables_tokens)
                    .map_err(|err| syn::Error::new(err.span(), err.to_string()))?;

                // Generate trait implementations inside block
                if types.len() == 1 {
                    quote! {
                        {
                            #schema_impl;
                            (::drizzle_rs::sqlite::Drizzle::new::<#schema_expr>(#conn), #(#types::default(),)*)
                        }
                    }
                } else {
                    quote! {
                        {
                            #schema_impl;
                            (::drizzle_rs::sqlite::Drizzle::new::<#schema_expr>(#conn), (#(#types::default(),)*))
                        }
                    }
                }
            }
            None => {
                // No tables specified, just use the schema type
                quote! {
                    ::drizzle_rs::sqlite::Drizzle::new::<#schema_expr>(#conn)
                }
            }
        }
    } else {
        // Fallback to old behavior when no schema type provided - generate schema automatically
        match &input.tables {
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
                            return Err(syn::Error::new_spanned(
                                single_table,
                                "Expected a table type",
                            ));
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
                        #[derive(Clone, Debug)]
                        pub struct EmptySchema;
                        ::drizzle_rs::sqlite::Drizzle::new::<EmptySchema>(#conn)
                    }
                }
            }
        }
    };

    Ok(output)
}
