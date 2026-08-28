//! Shared field assignment generation for SQLite `FromRow` derives.

use crate::paths;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Field;

/// Generate a field assignment through Drizzle's SQLite value codec.
pub fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
    supports_name_lookup: bool,
) -> TokenStream {
    let field_type = &field.ty;
    let drizzle_row = paths::sqlite::drizzle_row();
    let accessor = match field_name {
        Some(field_name) if supports_name_lookup => {
            let name = field_name.to_string();
            quote! {
                drizzle::sqlite::traits::DrizzleRowByName::get_column_by_name::<#field_type>(row, #name)
            }
        }
        _ => quote! {
            <_ as #drizzle_row>::get_column::<#field_type>(row, #idx)
        },
    };

    field_name.map_or_else(
        || quote!(#accessor?,),
        |field_name| quote!(#field_name: #accessor?,),
    )
}

/// Generate an offset-aware field assignment through Drizzle's SQLite value codec.
pub fn generate_field_assignment_with_index(
    idx: TokenStream,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    let field_type = &field.ty;
    let drizzle_row = paths::sqlite::drizzle_row();
    let accessor = quote! {
        <_ as #drizzle_row>::get_column::<#field_type>(row, #idx)
    };

    field_name.map_or_else(
        || quote!(#accessor?,),
        |field_name| quote!(#field_name: #accessor?,),
    )
}
