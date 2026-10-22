//! Shared field assignment generation for postgres-sync and tokio-postgres `FromRow` derive.
//!
//! Both drivers share one row type. `ArrayString`, `ArrayVec` and UUID fields
//! convert through drizzle's `DrizzleRowByIndex` (and so `FromPostgresValue`).
//! Every other field decodes through `driver_types::FieldDecode`, which uses a
//! custom `DrizzlePostgresColumn` codec when the type has one and the
//! driver's `FromSql` otherwise. Every path returns a decode error instead of
//! panicking.

use crate::postgres::field::TypeCategory;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Field;
use syn::ext::IdentExt;

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

fn extract_inner_type(ty: &syn::Type) -> &syn::Type {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
    {
        return inner_ty;
    }
    ty
}

/// The column index of `name`, or a `ConversionError` naming the column.
fn index_by_name(name: &str) -> TokenStream {
    let drizzle_error = crate::paths::core::drizzle_error();
    quote! {
        row.columns()
            .iter()
            .position(|column| column.name() == #name)
            .ok_or_else(|| #drizzle_error::ConversionError(
                ::std::format!("column '{}' not found", #name).into()
            ))?
    }
}

/// Decode `field`'s value from the column at `idx`, a `usize` expression.
fn field_value(idx: &TokenStream, field: &Field) -> TokenStream {
    let drizzle_error = crate::paths::core::drizzle_error();
    let ty = &field.ty;
    let category = TypeCategory::from_type(ty);

    if matches!(
        category,
        TypeCategory::ArrayString | TypeCategory::ArrayVec | TypeCategory::Uuid
    ) {
        return quote! {{
            use drizzle::postgres::traits::DrizzleRowByIndex;
            DrizzleRowByIndex::get_column::<#ty>(row, #idx)?
        }};
    }

    let decode = |target: &syn::Type| {
        quote! {{
            use drizzle::postgres::driver_types::{FieldDecode, ViaColumn as _, ViaFromSql as _};
            (&&FieldDecode::<#target>::new()).decode_field(row, #idx)?
        }}
    };

    if is_option_type(ty) {
        // The probe accepts every wire type, so a custom codec only sees
        // non-NULL cells.
        let inner = decode(extract_inner_type(ty));
        quote! {
            if row
                .try_get::<_, ::std::option::Option<drizzle::core::AnyNullProbe>>(#idx)
                .map_err(|e| #drizzle_error::ConversionError(e.to_string().into()))?
                .is_none()
            {
                ::std::option::Option::None
            } else {
                ::std::option::Option::Some(#inner)
            }
        }
    } else {
        decode(ty)
    }
}

fn assign(field_name: Option<&syn::Ident>, value: &TokenStream) -> TokenStream {
    field_name.map_or_else(
        || quote! { #value, },
        |field_name| quote! { #field_name: #value, },
    )
}

/// Generate one field's assignment for `TryFrom<&Row>`: by column name for a
/// named field, by index for a tuple field.
pub fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    let index = field_name.map_or_else(
        || quote!(#idx),
        |name| index_by_name(&name.unraw().to_string()),
    );
    let value = field_value(&quote!(__drizzle_idx), field);
    assign(
        field_name,
        &quote!({ let __drizzle_idx: usize = #index; #value }),
    )
}

/// Generate field assignment using an arbitrary index expression.
///
/// Used by `FromDrizzleRow::from_row_at` for offset-aware decoding.
pub fn generate_field_assignment_with_index_expr(
    idx_expr: &TokenStream,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> TokenStream {
    let value = field_value(&quote!(__drizzle_idx), field);
    assign(
        field_name,
        &quote!({ let __drizzle_idx: usize = #idx_expr; #value }),
    )
}

/// Generate named field assignment that decodes by name at top-level
/// (`offset == 0`) and by index for non-zero offsets.
///
/// This preserves named-struct behavior for top-level selects while keeping
/// offset-aware decoding correct when nested in larger selections.
pub fn generate_named_field_assignment_with_offset_fallback(
    idx: usize,
    field: &Field,
    field_name: &syn::Ident,
) -> TokenStream {
    let by_name = index_by_name(&field_name.unraw().to_string());
    let value = field_value(&quote!(__drizzle_idx), field);
    quote! {
        #field_name: {
            let __drizzle_idx: usize = if offset == 0 { #by_name } else { offset + #idx };
            #value
        },
    }
}
