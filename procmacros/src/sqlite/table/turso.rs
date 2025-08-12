use super::{FieldInfo, MacroContext};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, Result};

/// Generate TryFrom implementations for turso::Row for a table's models
pub(crate) fn generate_turso_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let MacroContext {
        field_infos,
        select_model_ident,
        update_model_ident,
        ..
    } = ctx;
    let (select, update, partial) = field_infos
        .iter()
        .map(|info| {
            let name = &info.ident;
            let column = &info.column_name;

            Ok((
                generate_field_from_row(info)?,
                generate_field_from_row(info)?,
                quote! { #name: row.get(#column).unwrap_or_default(), },
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&turso::Row> for #select_model_ident {
            type Error = turso::Error;

            fn try_from(row: &turso::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let partial_ident = format_ident!("Partial{}", select_model_ident);

    let partial_select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&turso::Row> for #partial_ident {
            type Error = turso::Error;

            fn try_from(row: &turso::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial)*
                })
            }
        }
    };

    // Insert models should not have TryFrom<Row> implementations since they're for
    // writing data TO the database, not reading FROM it

    let update_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&turso::Row> for #update_model_ident {
            type Error = turso::Error;

            fn try_from(row: &turso::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#update)*
                })
            }
        }
    };

    // Final return with all implementations combined
    Ok(quote! {
        #select_model_try_from_impl
        #partial_select_model_try_from_impl
        #update_model_try_from_impl
    })
}

/// Handles both standard types and conditional JSON deserialization for turso.
fn generate_field_from_row(info: &FieldInfo) -> Result<TokenStream> {
    let name = info.ident;
    let column_name = &info.column_name;

    if info.is_json && !cfg!(feature = "serde") {
        return Err(Error::new_spanned(
            info.ident,
            "JSON fields require the 'serde' feature to be enabled",
        ));
    } else if info.is_uuid {
        // Handle UUIDs as BLOB for turso - use row.get() for type-safe conversion
        Ok(quote! {
            #name: row.get(#column_name)?,
        })
    } else if info.is_json {
        // Handle JSON fields with serde - get as string then deserialize
        Ok(quote! {
            #name: {
                let json_str: String = row.get(#column_name)?;
                serde_json::from_str(&json_str)
                    .map_err(|e| turso::Error::Other(format!("JSON parse error: {}", e)))?
            },
        })
    } else {
        // Standard field types - use turso's type-safe get method
        Ok(quote! {
            #name: row.get(#column_name)?,
        })
    }
}