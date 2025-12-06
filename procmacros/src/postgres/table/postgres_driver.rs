use super::context::MacroContext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Generate TryFrom implementations for postgres rows for select/partial select models.
pub(crate) fn generate_postgres_driver_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let MacroContext {
        field_infos,
        select_model_ident,
        select_model_partial_ident,
        ..
    } = ctx;

    let mut select_field_inits = Vec::new();
    let mut partial_field_inits = Vec::new();

    for info in field_infos.iter() {
        let name = &info.ident;
        let ty = &info.ty;

        // Select model: use direct type
        select_field_inits.push(quote! {
            #name: row.get::<_, #ty>(stringify!(#name)),
        });

        // Partial select model: fields are Option<T>
        partial_field_inits.push(quote! {
            #name: row.get::<_, Option<#ty>>(stringify!(#name)),
        });
    }

    let partial_ident = select_model_partial_ident;

    Ok(quote! {
        impl ::std::convert::TryFrom<&::postgres::Row> for #select_model_ident {
            type Error = DrizzleError;

            fn try_from(row: &::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select_field_inits)*
                })
            }
        }

        impl ::std::convert::TryFrom<&::postgres::Row> for #partial_ident {
            type Error = DrizzleError;

            fn try_from(row: &::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial_field_inits)*
                })
            }
        }
    })
}
