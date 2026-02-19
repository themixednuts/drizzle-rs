use crate::common::has_json_attribute;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Result};

/// Generate rusqlite field assignment for FromRow derive
pub(crate) fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    if has_json_attribute(field) {
        let idx_or_name = if let Some(field_name) = field_name {
            let field_name_str = field_name.to_string();
            quote!(#field_name_str)
        } else {
            quote!(#idx)
        };
        return generate_json_field_assignment(field_name, idx_or_name);
    }

    let idx_or_name = if let Some(field_name) = field_name {
        let field_name_str = field_name.to_string();
        quote! { #field_name_str }
    } else {
        quote! { #idx }
    };

    let name = if let Some(field_name) = field_name {
        quote! {
            #field_name: row.get(#idx_or_name)?,
        }
    } else {
        quote! {
            row.get(#idx_or_name)?,
        }
    };
    Ok(name)
}

/// Generate rusqlite field assignment using an arbitrary index expression.
///
/// Used by FromDrizzleRow::from_row_at where columns are read at offset + idx.
pub(crate) fn generate_field_assignment_with_index_expr(
    idx_expr: TokenStream,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    if has_json_attribute(field) {
        return generate_json_field_assignment(field_name, idx_expr);
    }

    let name = if let Some(field_name) = field_name {
        quote! {
            #field_name: row.get(#idx_expr)?,
        }
    } else {
        quote! {
            row.get(#idx_expr)?,
        }
    };
    Ok(name)
}

/// Generate field assignment for `#[json]` fields, deserializing from a TEXT column.
fn generate_json_field_assignment(
    field_name: Option<&syn::Ident>,
    idx_expr: TokenStream,
) -> Result<TokenStream> {
    let get_json = quote! { row.get::<_, String>(#idx_expr)? };

    let accessor = quote! {
        {
            let json_str = #get_json;
            serde_json::from_str(&json_str)
                .map_err(|e| drizzle::error::DrizzleError::ConversionError(e.to_string().into()))
        }
    };

    if let Some(field_name) = field_name {
        Ok(quote! {
            #field_name: #accessor?,
        })
    } else {
        Ok(quote! {
            #accessor?,
        })
    }
}
