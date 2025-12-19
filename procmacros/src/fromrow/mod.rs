use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Expr, ExprPath, Field, Fields, Meta, Result};

#[cfg(feature = "libsql")]
mod libsql;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "rusqlite")]
mod rusqlite;
#[cfg(any(feature = "libsql", feature = "turso"))]
mod shared;
#[cfg(feature = "turso")]
mod turso;

/// Parse column reference from field attributes, looking for #[column(Table::field)]
fn parse_column_reference(field: &Field) -> Option<ExprPath> {
    for attr in &field.attrs {
        if let Some(ident) = attr.path().get_ident()
            && ident == "column"
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(Expr::Path(expr_path)) = syn::parse2::<Expr>(meta_list.tokens.clone())
        {
            return Some(expr_path);
        }
    }
    None
}

/// Helper to extract struct fields
fn extract_struct_fields(
    input: &DeriveInput,
) -> Result<(&syn::punctuated::Punctuated<Field, syn::token::Comma>, bool)> {
    let struct_name = &input.ident;
    match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => Ok((&fields.named, false)),
            Fields::Unnamed(fields) => Ok((&fields.unnamed, true)),
            Fields::Unit => Err(Error::new_spanned(
                struct_name,
                "FromRow cannot be derived for unit structs",
            )),
        },
        _ => Err(Error::new_spanned(
            struct_name,
            "FromRow can only be derived for structs",
        )),
    }
}

/// Generate SQLite-specific FromRow implementation (rusqlite, libsql, turso)
#[cfg(feature = "sqlite")]
pub(crate) fn generate_sqlite_from_row_impl(input: DeriveInput) -> Result<TokenStream> {
    use crate::paths::sqlite as sqlite_paths;

    let struct_name = &input.ident;
    let (fields, is_tuple) = extract_struct_fields(&input)?;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let to_sql = core_paths::to_sql();
    let token = core_paths::token();
    let drizzle_error = core_paths::drizzle_error();
    let sqlite_value = sqlite_paths::sqlite_value();

    let mut impl_blocks: Vec<TokenStream> = Vec::new();

    // Rusqlite implementation
    #[cfg(feature = "rusqlite")]
    {
        let field_assignments = if is_tuple {
            fields
                .iter()
                .enumerate()
                .map(|(idx, field)| rusqlite::generate_field_assignment(idx, field, None))
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    let field_name = field.ident.as_ref().unwrap();
                    rusqlite::generate_field_assignment(idx, field, Some(field_name))
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let rusqlite_impl = if is_tuple {
            quote! {
                impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #struct_name {
                    type Error = ::rusqlite::Error;

                    fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                        ::std::result::Result::Ok(Self(
                            #(#field_assignments)*
                        ))
                    }
                }
            }
        } else {
            quote! {
                impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #struct_name {
                    type Error = ::rusqlite::Error;

                    fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                        ::std::result::Result::Ok(Self {
                            #(#field_assignments)*
                        })
                    }
                }
            }
        };
        impl_blocks.push(rusqlite_impl);
    }

    // Turso implementation
    #[cfg(feature = "turso")]
    {
        let field_assignments = if is_tuple {
            fields
                .iter()
                .enumerate()
                .map(|(idx, field)| turso::generate_field_assignment(idx, field, None))
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    let field_name = field.ident.as_ref().unwrap();
                    turso::generate_field_assignment(idx, field, Some(field_name))
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let turso_impl = if is_tuple {
            quote! {
                impl ::std::convert::TryFrom<&::turso::Row> for #struct_name {
                    type Error = #drizzle_error;

                    fn try_from(row: &::turso::Row) -> ::std::result::Result<Self, Self::Error> {
                        ::std::result::Result::Ok(Self(
                            #(#field_assignments)*
                        ))
                    }
                }
            }
        } else {
            quote! {
                impl ::std::convert::TryFrom<&::turso::Row> for #struct_name {
                    type Error = #drizzle_error;

                    fn try_from(row: &::turso::Row) -> ::std::result::Result<Self, Self::Error> {
                        ::std::result::Result::Ok(Self {
                            #(#field_assignments)*
                        })
                    }
                }
            }
        };
        impl_blocks.push(turso_impl);
    }

    // Libsql implementation
    #[cfg(feature = "libsql")]
    {
        let field_assignments = if is_tuple {
            fields
                .iter()
                .enumerate()
                .map(|(idx, field)| libsql::generate_field_assignment(idx, field, None))
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    let field_name = field.ident.as_ref().unwrap();
                    libsql::generate_field_assignment(idx, field, Some(field_name))
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let libsql_impl = if is_tuple {
            quote! {
                impl ::std::convert::TryFrom<&::libsql::Row> for #struct_name {
                    type Error = #drizzle_error;

                    fn try_from(row: &::libsql::Row) -> ::std::result::Result<Self, Self::Error> {
                        ::std::result::Result::Ok(Self(
                            #(#field_assignments)*
                        ))
                    }
                }
            }
        } else {
            quote! {
                impl ::std::convert::TryFrom<&::libsql::Row> for #struct_name {
                    type Error = #drizzle_error;

                    fn try_from(row: &::libsql::Row) -> ::std::result::Result<Self, Self::Error> {
                        ::std::result::Result::Ok(Self {
                            #(#field_assignments)*
                        })
                    }
                }
            }
        };
        impl_blocks.push(libsql_impl);
    }

    // Generate ToSQL implementation for SQLite FromRow structs
    let tosql_impl = if is_tuple {
        quote! {}
    } else {
        let column_specs = fields
            .iter()
            .map(|field| {
                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.to_string();

                if let Some(column_ref) = parse_column_reference(field) {
                    quote! {
                        columns.push(#to_sql::to_sql(&#column_ref).alias(#field_name_str));
                    }
                } else {
                    quote! {
                        columns.push(#sql::raw(#field_name_str));
                    }
                }
            })
            .collect::<Vec<_>>();

        quote! {
            impl<'a> #to_sql<'a, #sqlite_value<'a>> for #struct_name {
                fn to_sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                    let mut columns = ::std::vec::Vec::new();
                    #(#column_specs)*
                    #sql::join(columns, #token::COMMA)
                }
            }
        }
    };

    Ok(quote! {
        #(#impl_blocks)*
        #tosql_impl
    })
}

/// Generate PostgreSQL-specific FromRow implementation (postgres-sync, tokio-postgres)
#[cfg(feature = "postgres")]
pub(crate) fn generate_postgres_from_row_impl(input: DeriveInput) -> Result<TokenStream> {
    use crate::paths::postgres as postgres_paths;

    let struct_name = &input.ident;
    let (fields, is_tuple) = extract_struct_fields(&input)?;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let to_sql = core_paths::to_sql();
    let token = core_paths::token();
    let drizzle_error = core_paths::drizzle_error();
    let postgres_value = postgres_paths::postgres_value();

    let field_assignments = if is_tuple {
        fields
            .iter()
            .enumerate()
            .map(|(idx, field)| postgres::generate_field_assignment(idx, field, None))
            .collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        fields
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                let field_name = field.ident.as_ref().unwrap();
                postgres::generate_field_assignment(idx, field, Some(field_name))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?
    };

    let struct_construct = if is_tuple {
        quote! {
            ::std::result::Result::Ok(Self(
                #(#field_assignments)*
            ))
        }
    } else {
        quote! {
            ::std::result::Result::Ok(Self {
                #(#field_assignments)*
            })
        }
    };

    // Generate ToSQL implementation for PostgreSQL FromRow structs
    let tosql_impl = if is_tuple {
        quote! {}
    } else {
        let column_specs = fields
            .iter()
            .map(|field| {
                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.to_string();

                if let Some(column_ref) = parse_column_reference(field) {
                    quote! {
                        columns.push(#to_sql::to_sql(&#column_ref).alias(#field_name_str));
                    }
                } else {
                    quote! {
                        columns.push(#sql::raw(#field_name_str));
                    }
                }
            })
            .collect::<Vec<_>>();

        quote! {
            impl<'a> #to_sql<'a, #postgres_value<'a>> for #struct_name {
                fn to_sql(&self) -> #sql<'a, #postgres_value<'a>> {
                    let mut columns = ::std::vec::Vec::new();
                    #(#column_specs)*
                    #sql::join(columns, #token::COMMA)
                }
            }
        }
    };

    // Generate the implementations with proper conditional compilation
    // to avoid duplicate implementations (postgres::Row is tokio_postgres::Row)
    Ok(quote! {
        // When tokio-postgres is enabled, use tokio_postgres::Row
        // This covers both "tokio-postgres only" and "both features enabled" cases
        #[cfg(feature = "tokio-postgres")]
        impl ::std::convert::TryFrom<&::tokio_postgres::Row> for #struct_name {
            type Error = #drizzle_error;

            fn try_from(row: &::tokio_postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                #struct_construct
            }
        }

        // When only postgres-sync is enabled (without tokio-postgres), use postgres::Row
        #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
        impl ::std::convert::TryFrom<&::postgres::Row> for #struct_name {
            type Error = #drizzle_error;

            fn try_from(row: &::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                #struct_construct
            }
        }

        #tosql_impl
    })
}
