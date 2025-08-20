use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Result};

/// Generate rusqlite field assignment for FromRow derive
pub(crate) fn generate_field_assignment(
    idx: usize,
    _field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    let name = if let Some(field_name) = field_name {
        let field_name_str = field_name.to_string();
        quote! {
            #field_name: row.get(#field_name_str)?,
        }
    } else {
        quote! {
            row.get(#idx)?,
        }
    };
    Ok(name)
}
