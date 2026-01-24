//! Type mapping utilities for converting Rust types to SQL type markers.
//!
//! This module provides functions to determine the appropriate `DataType` and
//! `Nullability` markers for column types based on their Rust types.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::common::{
    is_option_type, option_inner_type, type_is_array_u8, type_is_bool, type_is_byte_slice,
    type_is_chrono_date, type_is_chrono_time, type_is_datetime_tz, type_is_float, type_is_int,
    type_is_json_value, type_is_naive_datetime, type_is_string_like, type_is_uuid, type_is_vec_u8,
};
use crate::paths::core as core_paths;

/// Determines the SQL DataType marker for a given Rust type.
///
/// Maps common Rust types to their corresponding `drizzle_core::types` markers.
/// Unknown types fall back to `Any` for backward compatibility.
pub fn rust_type_to_sql_type(ty: &Type) -> TokenStream {
    let types = core_paths::types();
    let ty = option_inner_type(ty).unwrap_or(ty);

    if type_is_int(ty, "i8") || type_is_int(ty, "i16") || type_is_int(ty, "u8") {
        return quote!(#types::SmallInt);
    }
    if type_is_int(ty, "i32") || type_is_int(ty, "u16") {
        return quote!(#types::Int);
    }
    if type_is_int(ty, "i64")
        || type_is_int(ty, "isize")
        || type_is_int(ty, "u32")
        || type_is_int(ty, "u64")
        || type_is_int(ty, "usize")
    {
        return quote!(#types::BigInt);
    }
    if type_is_float(ty, "f32") {
        return quote!(#types::Float);
    }
    if type_is_float(ty, "f64") {
        return quote!(#types::Double);
    }
    if type_is_bool(ty) {
        return quote!(#types::Bool);
    }
    if type_is_string_like(ty) {
        return quote!(#types::Text);
    }
    if type_is_vec_u8(ty) || type_is_byte_slice(ty) || type_is_array_u8(ty) {
        return quote!(#types::Bytes);
    }
    if type_is_uuid(ty) {
        return quote!(#types::Uuid);
    }
    if type_is_naive_datetime(ty) {
        return quote!(#types::Timestamp);
    }
    if type_is_datetime_tz(ty) {
        return quote!(#types::TimestampTz);
    }
    if type_is_chrono_date(ty) {
        return quote!(#types::Date);
    }
    if type_is_chrono_time(ty) {
        return quote!(#types::Time);
    }
    if type_is_json_value(ty) {
        return quote!(#types::Json);
    }

    quote!(#types::Any)
}

/// Determines the nullability marker for a given Rust type.
///
/// Returns `Null` for `Option<T>` types, `NonNull` otherwise.
pub fn rust_type_to_nullability(ty: &Type) -> TokenStream {
    let expr = core_paths::expr();
    if is_option_type(ty) {
        quote!(#expr::Null)
    } else {
        quote!(#expr::NonNull)
    }
}

/// Generates an Expr trait implementation for a column type.
pub fn generate_expr_impl(
    struct_ident: &proc_macro2::Ident,
    value_type: TokenStream,
    sql_type: TokenStream,
    sql_nullable: TokenStream,
) -> TokenStream {
    let expr = core_paths::expr();

    quote! {
        impl<'a> #expr::Expr<'a, #value_type<'a>> for #struct_ident {
            type SQLType = #sql_type;
            type Nullable = #sql_nullable;
            type Aggregate = #expr::Scalar;
        }
    }
}

/// Generates arithmetic operator implementations for a numeric column type.
///
/// This generates `Add`, `Sub`, `Mul`, `Div`, `Rem`, and `Neg` implementations
/// so users can write `column + 5` directly instead of `lit(column) + 5`.
///
/// Returns wrapper types (`ColumnBinOp`, `ColumnNeg`) that implement `ToSQL<'a, V>`
/// for any lifetime, allowing seamless use with query builders.
pub fn generate_arithmetic_ops(
    struct_ident: &proc_macro2::Ident,
    _value_type: TokenStream,
    _sql_type: TokenStream,
    _sql_nullable: TokenStream,
) -> TokenStream {
    let expr = core_paths::expr();

    quote! {
        // Add operator: column + rhs
        impl<Rhs: ::core::marker::Copy> ::core::ops::Add<Rhs> for #struct_ident {
            type Output = #expr::ColumnBinOp<#struct_ident, Rhs, #expr::OpAdd>;

            fn add(self, rhs: Rhs) -> Self::Output {
                #expr::ColumnBinOp::new(self, rhs)
            }
        }

        // Sub operator: column - rhs
        impl<Rhs: ::core::marker::Copy> ::core::ops::Sub<Rhs> for #struct_ident {
            type Output = #expr::ColumnBinOp<#struct_ident, Rhs, #expr::OpSub>;

            fn sub(self, rhs: Rhs) -> Self::Output {
                #expr::ColumnBinOp::new(self, rhs)
            }
        }

        // Mul operator: column * rhs
        impl<Rhs: ::core::marker::Copy> ::core::ops::Mul<Rhs> for #struct_ident {
            type Output = #expr::ColumnBinOp<#struct_ident, Rhs, #expr::OpMul>;

            fn mul(self, rhs: Rhs) -> Self::Output {
                #expr::ColumnBinOp::new(self, rhs)
            }
        }

        // Div operator: column / rhs
        impl<Rhs: ::core::marker::Copy> ::core::ops::Div<Rhs> for #struct_ident {
            type Output = #expr::ColumnBinOp<#struct_ident, Rhs, #expr::OpDiv>;

            fn div(self, rhs: Rhs) -> Self::Output {
                #expr::ColumnBinOp::new(self, rhs)
            }
        }

        // Rem operator: column % rhs
        impl<Rhs: ::core::marker::Copy> ::core::ops::Rem<Rhs> for #struct_ident {
            type Output = #expr::ColumnBinOp<#struct_ident, Rhs, #expr::OpRem>;

            fn rem(self, rhs: Rhs) -> Self::Output {
                #expr::ColumnBinOp::new(self, rhs)
            }
        }

        // Neg operator: -column
        impl ::core::ops::Neg for #struct_ident {
            type Output = #expr::ColumnNeg<#struct_ident>;

            fn neg(self) -> Self::Output {
                #expr::ColumnNeg::new(self)
            }
        }
    }
}

/// Checks if a SQL type marker is numeric and can have arithmetic operators.
pub fn is_numeric_sql_type(ty: &Type) -> bool {
    let ty = option_inner_type(ty).unwrap_or(ty);
    matches!(
        ty,
        _ if type_is_int(ty, "i8")
            || type_is_int(ty, "i16")
            || type_is_int(ty, "u8")
            || type_is_int(ty, "i32")
            || type_is_int(ty, "u16")
            || type_is_int(ty, "i64")
            || type_is_int(ty, "isize")
            || type_is_int(ty, "u32")
            || type_is_int(ty, "u64")
            || type_is_int(ty, "usize")
            || type_is_float(ty, "f32")
            || type_is_float(ty, "f64")
    )
}
