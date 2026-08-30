use proc_macro2::TokenStream;
use quote::quote;
use syn::Field;

pub fn generate_field_assignment(
    idx: usize,
    _field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    let index = field_name.map_or_else(
        || quote!(#idx),
        |name| {
            let name = name.to_string();
            quote!(#name)
        },
    );

    field_name.map_or_else(
        || quote!(row.get(#index)?,),
        |name| quote!(#name: row.get(#index)?,),
    )
}

pub fn generate_field_assignment_with_index_expr(
    index: &TokenStream,
    _field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    field_name.map_or_else(
        || quote!(row.get(#index)?,),
        |name| quote!(#name: row.get(#index)?,),
    )
}
