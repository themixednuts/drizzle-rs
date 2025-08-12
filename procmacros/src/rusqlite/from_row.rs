use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result};

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
        // For named structs, use field name-based access
        fields.iter().map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            
            quote! {
                #field_name: row.get(#field_name_str)?,
            }
        }).collect::<Vec<_>>()
    };

    // Generate the implementation based on struct type
    let impl_block = if is_tuple {
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

    Ok(impl_block)
}