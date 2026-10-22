//! libsql driver implementation for `SQLite` table macro.
//!
//! Generates `TryFrom` implementations for `libsql::Row` using the shared driver infrastructure.

use super::drivers;
use super::{FieldInfo, MacroContext};
use crate::common::is_option_type;
use crate::paths;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

// =============================================================================
// Public API
// =============================================================================

/// Generate `TryFrom` implementations for `libsql::Row` for a table's models
pub fn generate_libsql_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let drizzle_error = paths::core::drizzle_error();
    let row_column_list = paths::core::row_column_list();
    let type_set_nil = paths::core::type_set_nil();
    let type_set_cons = paths::core::type_set_cons();
    let MacroContext {
        field_infos,
        select_model_ident,
        ..
    } = ctx;

    let select: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(i, info)| generate_field_from_row_for_select(i, info))
        .collect::<Result<Vec<_>>>()?;

    let from_drizzle_select: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(i, info)| generate_field_from_row_for_select_with_index(&quote!(offset + #i), info))
        .collect::<Result<Vec<_>>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&drizzle::sqlite::libsql::Row> for #select_model_ident {
            type Error = #drizzle_error;

            fn try_from(row: &drizzle::sqlite::libsql::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let field_count = field_infos.len();
    let mut column_list = quote!(#type_set_nil);
    for info in field_infos.iter().rev() {
        let select_ty = info.get_select_type();
        column_list = quote!(#type_set_cons<#select_ty, #column_list>);
    }
    let from_drizzle_row_impl = quote! {
        impl drizzle::core::FromDrizzleRow<drizzle::sqlite::libsql::Row> for #select_model_ident {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(row: &drizzle::sqlite::libsql::Row, offset: usize) -> ::std::result::Result<Self, #drizzle_error> {
                Ok(Self {
                    #(#from_drizzle_select)*
                })
            }
        }
    };
    let row_column_list_impl = quote! {
        impl #row_column_list<drizzle::sqlite::libsql::Row> for #select_model_ident {
            type Columns = #column_list;
        }
    };

    Ok(quote! {
        #select_model_try_from_impl
        #from_drizzle_row_impl
        #row_column_list_impl
    })
}

// =============================================================================
// Field Conversion
// =============================================================================

fn generate_field_from_row_for_select(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    generate_field_from_row_for_select_with_index(&quote!(#idx), info)
}

fn generate_field_from_row_for_select_with_index(
    idx: &TokenStream,
    info: &FieldInfo,
) -> Result<TokenStream> {
    let select_type = info.get_select_type();
    let is_optional = syn::parse2::<syn::Type>(select_type)
        .map_or(info.is_nullable && !info.has_default, |ty| {
            is_option_type(&ty)
        });
    drivers::generate_field_conversion_with_index(idx, info, is_optional)
}
