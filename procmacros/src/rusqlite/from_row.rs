use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result};

/// Generate a `TryFrom<&Row<'_>>` implementation for a struct using field name-based column access
pub(crate) fn generate_from_row_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    
    // Check if this is a struct
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new_spanned(
                    struct_name,
                    "FromRow can only be derived for structs with named fields",
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

    // Generate field assignments using field names as column names
    let field_assignments = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        
        quote! {
            #field_name: row.get(#field_name_str)?,
        }
    });

    // Generate the implementation
    let impl_block = quote! {
        impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #struct_name {
            type Error = ::rusqlite::Error;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#field_assignments)*
                })
            }
        }
    };

    Ok(impl_block)
}