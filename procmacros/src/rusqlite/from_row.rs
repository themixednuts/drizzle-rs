use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result, Meta, Field, Expr, ExprPath};

/// Parse column reference from field attributes, looking for #[column(Table::field)]
fn parse_column_reference(field: &Field) -> Option<ExprPath> {
    for attr in &field.attrs {
        if let Some(ident) = attr.path().get_ident() {
            if ident == "column" {
                if let Meta::List(meta_list) = &attr.meta {
                    if let Ok(Expr::Path(expr_path)) = syn::parse2::<Expr>(meta_list.tokens.clone()) {
                        return Some(expr_path);
                    }
                }
            }
        }
    }
    None
}

/// Generate a `TryFrom<&Row<'_>>` implementation for a struct using field name-based or index-based column access
pub(crate) fn generate_from_row_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    
    // Check if this is a struct and determine field type
    let (fields, is_tuple) = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => (&fields.named, false),
            Fields::Unnamed(fields) => (&fields.unnamed, true),
            Fields::Unit => {
                return Err(Error::new_spanned(
                    struct_name,
                    "FromRow cannot be derived for unit structs",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(
                struct_name,
                "FromRow can only be derived for structs",
            ));
        }
    };

    // Generate field assignments
    let field_assignments = if is_tuple {
        // For tuple structs, use index-based access
        fields.iter().enumerate().map(|(idx, _field)| {
            quote! {
                row.get(#idx)?,
            }
        }).collect::<Vec<_>>()
    } else {
        // For named structs, use field name as the column alias
        fields.iter().map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            
            quote! {
                #field_name: row.get(#field_name_str)?,
            }
        }).collect::<Vec<_>>()
    };

    // Generate implementations for all drivers
    let mut impl_blocks = Vec::new();
    
    // Rusqlite implementation
    #[cfg(feature = "rusqlite")]
    {
        let rusqlite_impl = if is_tuple {
            quote! {
                impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #struct_name {
                    type Error = ::rusqlite::Error;

                    fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                        Ok(Self(
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
                        Ok(Self {
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
        let turso_field_assignments = if is_tuple {
            // For tuple structs, use index-based access
            fields.iter().enumerate().map(|(idx, _field)| {
                quote! {
                    row.get_value(#idx)?.as_text().unwrap_or_default().parse()?,
                }
            }).collect::<Vec<_>>()
        } else {
            // For named structs, still use index-based access (turso doesn't support name-based)
            fields.iter().enumerate().map(|(idx, field)| {
                let field_name = field.ident.as_ref().unwrap();
                quote! {
                    #field_name: row.get_value(#idx)?.as_text().unwrap_or_default().parse()?,
                }
            }).collect::<Vec<_>>()
        };
        
        let turso_impl = if is_tuple {
            quote! {
                impl ::std::convert::TryFrom<&::turso::Row> for #struct_name {
                    type Error = ::drizzle_rs::error::DrizzleError;

                    fn try_from(row: &::turso::Row) -> ::std::result::Result<Self, Self::Error> {
                        Ok(Self(
                            #(#turso_field_assignments)*
                        ))
                    }
                }
            }
        } else {
            quote! {
                impl ::std::convert::TryFrom<&::turso::Row> for #struct_name {
                    type Error = ::drizzle_rs::error::DrizzleError;

                    fn try_from(row: &::turso::Row) -> ::std::result::Result<Self, Self::Error> {
                        Ok(Self {
                            #(#turso_field_assignments)*
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
        let libsql_field_assignments = if is_tuple {
            // For tuple structs, use index-based access
            fields.iter().enumerate().map(|(idx, _field)| {
                let idx = idx as i32; // libsql uses i32 for indices
                quote! {
                    row.get_value(#idx)?.as_text().unwrap_or_default().parse()?,
                }
            }).collect::<Vec<_>>()
        } else {
            // For named structs, still use index-based access (libsql doesn't support name-based)
            fields.iter().enumerate().map(|(idx, field)| {
                let idx = idx as i32; // libsql uses i32 for indices
                let field_name = field.ident.as_ref().unwrap();
                quote! {
                    #field_name: row.get_value(#idx)?.as_text().unwrap_or_default().parse()?,
                }
            }).collect::<Vec<_>>()
        };
        
        let libsql_impl = if is_tuple {
            quote! {
                impl ::std::convert::TryFrom<&::libsql::Row> for #struct_name {
                    type Error = ::drizzle_rs::error::DrizzleError;

                    fn try_from(row: &::libsql::Row) -> ::std::result::Result<Self, Self::Error> {
                        Ok(Self(
                            #(#libsql_field_assignments)*
                        ))
                    }
                }
            }
        } else {
            quote! {
                impl ::std::convert::TryFrom<&::libsql::Row> for #struct_name {
                    type Error = ::drizzle_rs::error::DrizzleError;

                    fn try_from(row: &::libsql::Row) -> ::std::result::Result<Self, Self::Error> {
                        Ok(Self {
                            #(#libsql_field_assignments)*
                        })
                    }
                }
            }
        };
        impl_blocks.push(libsql_impl);
    }
    
    let impl_block = quote! {
        #(#impl_blocks)*
    };

    // Generate ToSQL implementation for FromRow structs
    let tosql_impl = if is_tuple {
        // For tuple structs, can't easily generate ToSQL as we don't have field names
        quote! {}
    } else {
        let column_specs = fields.iter().map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            
            // Check for column reference attribute
            if let Some(column_ref) = parse_column_reference(field) {
                // Use the column reference with alias
                quote! {
                    columns.push(#column_ref.to_sql().alias(#field_name_str));
                }
            } else {
                // Fallback to field name as raw SQL
                quote! {
                    columns.push(::drizzle_rs::core::SQL::raw(#field_name_str));
                }
            }
        }).collect::<Vec<_>>();

        quote! {
            impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #struct_name {
                fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                    let mut columns = Vec::new();
                    #(#column_specs)*
                    ::drizzle_rs::core::SQL::join(columns, ", ")
                }
            }
            
            impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #struct_name {
                fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
                    // This is a simplified implementation since we don't have access to table info
                    // In practice, FromRow structs used with .select() will typically be generated
                    // by the table macro which provides proper column info
                    Box::new([])
                }
                
                fn values(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                    // FromRow structs are primarily for result mapping, not value generation
                    ::drizzle_rs::core::SQL::empty()
                }
            }
        }
    };
    
    Ok(quote! {
        #impl_block
        #tosql_impl
    })
}