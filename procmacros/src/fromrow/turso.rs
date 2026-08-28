use super::shared;
use proc_macro2::TokenStream;
use syn::Field;

pub fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    shared::generate_field_assignment(idx, field, field_name, false)
}

pub fn generate_field_assignment_with_index_expr(
    idx: TokenStream,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    shared::generate_field_assignment_with_index(idx, field, field_name)
}
