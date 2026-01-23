//! Type mapping utilities for converting Rust types to SQL type markers.
//!
//! This module provides functions to determine the appropriate `DataType` and
//! `Nullability` markers for column types based on their Rust types.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::paths::core as core_paths;

/// Determines the SQL DataType marker for a given Rust type.
///
/// Maps common Rust types to their corresponding `drizzle_core::types` markers.
/// Unknown types fall back to `Any` for backward compatibility.
pub fn rust_type_to_sql_type(ty: &Type) -> TokenStream {
    let types = core_paths::types();
    let ty_str = quote!(#ty).to_string().replace(' ', "");

    match ty_str.as_str() {
        // Small integers
        "i8" | "i16" | "u8" => quote!(#types::SmallInt),

        // Regular integers
        "i32" | "u16" => quote!(#types::Int),

        // Big integers
        "i64" | "isize" | "u32" | "u64" | "usize" => quote!(#types::BigInt),

        // Floating point
        "f32" => quote!(#types::Float),
        "f64" => quote!(#types::Double),

        // Boolean
        "bool" => quote!(#types::Bool),

        // Text types
        "String" | "&str" | "&'staticstr" | "&'astr" => quote!(#types::Text),

        // Handle Option<T> - extract inner type
        s if s.starts_with("Option<") => {
            if let Type::Path(type_path) = ty
                && let Some(segment) = type_path.path.segments.last()
                && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            {
                return rust_type_to_sql_type(inner);
            }
            quote!(#types::Any)
        }

        // Binary data
        s if s.starts_with("Vec<u8>") || s.contains("&[u8]") => quote!(#types::Bytes),

        // UUID
        s if s.contains("Uuid") => quote!(#types::Uuid),

        // Date/Time types
        s if s.contains("NaiveDateTime") => quote!(#types::Timestamp),
        s if s.contains("DateTime") => quote!(#types::TimestampTz),
        s if s.contains("NaiveDate") => quote!(#types::Date),
        s if s.contains("NaiveTime") => quote!(#types::Time),

        // JSON types
        s if s.contains("serde_json") || s.contains("Value") => quote!(#types::Json),

        // Fallback for unknown types
        _ => quote!(#types::Any),
    }
}

/// Determines the nullability marker for a given Rust type.
///
/// Returns `Null` for `Option<T>` types, `NonNull` otherwise.
pub fn rust_type_to_nullability(ty: &Type) -> TokenStream {
    let expr = core_paths::expr();
    let ty_str = quote!(#ty).to_string().replace(' ', "");

    if ty_str.starts_with("Option<") {
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
    let ty_str = quote!(#ty).to_string().replace(' ', "");

    matches!(
        ty_str.as_str(),
        // Small integers
        "i8" | "i16" | "u8" |
        // Regular integers
        "i32" | "u16" |
        // Big integers
        "i64" | "isize" | "u32" | "u64" | "usize" |
        // Floating point
        "f32" | "f64"
    )
}
