//! Shared field assignment generation for postgres-sync and tokio-postgres FromRow derive.
//!
//! Both drivers use the shared DrizzleRow::get_column interface for unified type conversion
//! via the FromPostgresValue trait, while standard types use the native driver's get method.

use crate::postgres::field::TypeCategory;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Result};

/// Driver-specific configuration for PostgreSQL row access
#[allow(dead_code)]
pub(crate) trait DriverConfig {
    /// Get the row type for this driver
    fn row_type() -> TokenStream;

    /// Get the feature name for conditional compilation
    fn feature_name() -> &'static str;
}

/// Configuration for postgres-sync driver
#[allow(dead_code)]
pub(crate) struct PostgresSyncDriver;

impl DriverConfig for PostgresSyncDriver {
    fn row_type() -> TokenStream {
        quote!(::postgres::Row)
    }

    fn feature_name() -> &'static str {
        "postgres-sync"
    }
}

/// Configuration for tokio-postgres driver
#[allow(dead_code)]
pub(crate) struct TokioPostgresDriver;

impl DriverConfig for TokioPostgresDriver {
    fn row_type() -> TokenStream {
        quote!(::tokio_postgres::Row)
    }

    fn feature_name() -> &'static str {
        "tokio-postgres"
    }
}

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

/// Generate field assignment using the driver-agnostic approach.
///
/// For special types like ArrayVec/ArrayString, uses DrizzleRow::get_column
/// with FromPostgresValue trait. For standard types, uses the native driver's get method.
pub(crate) fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    let category = TypeCategory::from_type(&field.ty);

    let idx_or_name = quote! { #idx };

    let target_type = extract_inner_type(&field.ty);
    let is_optional = is_option_type(&field.ty);

    // Determine if we need special handling via FromPostgresValue trait
    let needs_from_postgres_value = matches!(
        category,
        TypeCategory::ArrayString | TypeCategory::ArrayVec | TypeCategory::Uuid
    );

    let assignment = if needs_from_postgres_value {
        // Use capability-specific row traits with FromPostgresValue conversion
        if is_optional {
            quote! {
                {
                    use drizzle::postgres::traits::DrizzleRowByIndex;
                    DrizzleRowByIndex::get_column::<Option<#target_type>>(row, #idx_or_name)?
                }
            }
        } else {
            quote! {
                {
                    use drizzle::postgres::traits::DrizzleRowByIndex;
                    DrizzleRowByIndex::get_column::<#target_type>(row, #idx_or_name)?
                }
            }
        }
    } else {
        // Use native driver's get method
        let ty = &field.ty;
        quote! {
            row.get::<_, #ty>(#idx_or_name)
        }
    };

    let name = if let Some(field_name) = field_name {
        quote! {
            #field_name: #assignment,
        }
    } else {
        quote! {
            #assignment,
        }
    };
    Ok(name)
}
