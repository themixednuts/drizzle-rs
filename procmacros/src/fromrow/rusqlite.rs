use crate::common::has_json_attribute;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Field;

/// Generate rusqlite field assignment for `FromRow` derive
pub fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    if has_json_attribute(field) {
        let idx_or_name = field_name.map_or_else(
            || quote!(#idx),
            |field_name| {
                let field_name_str = field_name.to_string();
                quote!(#field_name_str)
            },
        );
        return generate_json_field_assignment(field_name, &idx_or_name);
    }

    let idx_or_name = field_name.map_or_else(
        || quote! { #idx },
        |field_name| {
            let field_name_str = field_name.to_string();
            quote! { #field_name_str }
        },
    );

    field_name.map_or_else(
        || {
            quote! {
                row.get(#idx_or_name)?,
            }
        },
        |field_name| {
            quote! {
                #field_name: row.get(#idx_or_name)?,
            }
        },
    )
}

/// Generate rusqlite field assignment using an arbitrary index expression.
///
/// Used by `FromDrizzleRow::from_row_at` where columns are read at offset + idx.
pub fn generate_field_assignment_with_index_expr(
    idx_expr: &TokenStream,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    if has_json_attribute(field) {
        return generate_json_field_assignment(field_name, idx_expr);
    }

    field_name.map_or_else(
        || {
            quote! {
                row.get(#idx_expr)?,
            }
        },
        |field_name| {
            quote! {
                #field_name: row.get(#idx_expr)?,
            }
        },
    )
}

/// Generate field assignment for `#[json]` fields, deserializing from a TEXT column.
fn generate_json_field_assignment(
    field_name: Option<&syn::Ident>,
    idx_expr: &TokenStream,
) -> TokenStream {
    let get_json = quote! { row.get::<_, String>(#idx_expr)? };

    let accessor = quote! {
        {
            let json_str = #get_json;
            serde_json::from_str(&json_str)
                .map_err(|e| drizzle::error::DrizzleError::ConversionError(e.to_string().into()))
        }
    };

    field_name.map_or_else(
        || {
            quote! {
                #accessor?,
            }
        },
        |field_name| {
            quote! {
                #field_name: #accessor?,
            }
        },
    )
}
