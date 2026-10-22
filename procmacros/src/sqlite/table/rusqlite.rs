//! rusqlite driver implementation for `SQLite` table macro.
//!
//! Generates `TryFrom` implementations for `rusqlite::Row` using the `FromSQLiteValue` trait
//! (JSON fields through `drizzle::core::Json<T>`), shared with libsql and turso.

use super::{FieldInfo, MacroContext};
use crate::paths;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

// =============================================================================
// Public API
// =============================================================================

/// Generate `TryFrom` implementations for `rusqlite::Row` for a table's models
pub fn generate_rusqlite_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let drizzle_error = paths::core::drizzle_error();
    let row_column_list = paths::core::row_column_list();
    let type_set_nil = paths::core::type_set_nil();
    let type_set_cons = paths::core::type_set_cons();
    let MacroContext {
        field_infos,
        select_model_ident,
        ..
    } = ctx;

    let (select, partial) = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| {
            Ok((
                generate_field_from_row(idx, info)?,
                generate_partial_field_from_row(idx, info),
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&drizzle::sqlite::rusqlite::Row<'_>> for #select_model_ident {
            type Error = #drizzle_error;

            fn try_from(row: &drizzle::sqlite::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let partial_ident = format_ident!("Partial{}", select_model_ident);

    let partial_select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&drizzle::sqlite::rusqlite::Row<'_>> for #partial_ident {
            type Error = #drizzle_error;

            fn try_from(row: &drizzle::sqlite::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial)*
                })
            }
        }
    };

    let from_drizzle_row_fields: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| {
            let is_select_optional = syn::parse2::<syn::Type>(info.get_select_type())
                .map_or(info.is_nullable && !info.has_default, |ty| {
                    crate::common::is_option_type(&ty)
                });
            super::drivers::generate_field_conversion_with_index(
                &quote!(offset + #idx),
                info,
                is_select_optional,
            )
        })
        .collect::<Result<_>>()?;

    let field_count = field_infos.len();
    let mut column_list = quote!(#type_set_nil);
    for info in field_infos.iter().rev() {
        let select_ty = info.get_select_type();
        column_list = quote!(#type_set_cons<#select_ty, #column_list>);
    }
    let from_drizzle_row_impl = quote! {
        impl<'__drizzle_r> drizzle::core::FromDrizzleRow<drizzle::sqlite::rusqlite::Row<'__drizzle_r>> for #select_model_ident {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(row: &drizzle::sqlite::rusqlite::Row<'__drizzle_r>, offset: usize) -> ::std::result::Result<Self, #drizzle_error> {
                Ok(Self {
                    #(#from_drizzle_row_fields)*
                })
            }
        }
    };
    let row_column_list_impl = quote! {
        impl<'__drizzle_r> #row_column_list<drizzle::sqlite::rusqlite::Row<'__drizzle_r>> for #select_model_ident {
            type Columns = #column_list;
        }
    };

    Ok(quote! {
        #select_model_try_from_impl
        #partial_select_model_try_from_impl
        #from_drizzle_row_impl
        #row_column_list_impl
    })
}

// =============================================================================
// Field Conversion Generators
// =============================================================================

/// Generate field conversion for `SelectModel`. `TryFrom<&Row>` decodes
/// exactly like `FromDrizzleRow`, through the field type's `FromSQLiteValue`
/// codec.
fn generate_field_from_row(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let is_select_optional = syn::parse2::<syn::Type>(info.get_select_type())
        .map_or(info.is_nullable && !info.has_default, |ty| {
            crate::common::is_option_type(&ty)
        });
    super::drivers::generate_field_conversion_with_index(&quote!(#idx), info, is_select_optional)
}

/// Generate field conversion for `PartialSelectModel` (all fields are
/// `Option<T>`). A column past the end of the row is absent and decodes as
/// `None`; a NULL cell is `None`; a value that fails to decode is an error.
fn generate_partial_field_from_row(idx: usize, info: &FieldInfo) -> TokenStream {
    let name = info.ident;
    let base_type = info.base_type;
    let target = if info.is_json_column() {
        quote!(::std::option::Option<drizzle::core::Json<#base_type>>)
    } else {
        quote!(::std::option::Option<#base_type>)
    };
    let unwrap_json = if info.is_json_column() {
        quote!(.map(drizzle::core::Json::into_inner))
    } else {
        quote!()
    };

    quote! {
        #name: if #idx < row.as_ref().column_count() {
            use drizzle::sqlite::traits::DrizzleRowByIndex;
            DrizzleRowByIndex::get_column::<#target>(row, #idx)? #unwrap_json
        } else {
            None
        },
    }
}
