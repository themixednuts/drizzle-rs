//! Type mapping utilities for converting schema column types to SQL markers.
//!
//! Expression SQL markers are derived from declared database column types.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

#[cfg(feature = "postgres")]
use crate::postgres::field::PostgreSQLType;
#[cfg(feature = "sqlite")]
use crate::sqlite::field::SQLiteType;

use crate::common::is_option_type;
use crate::paths::core as core_paths;

#[cfg(feature = "sqlite")]
pub fn sqlite_column_type_to_sql_type(column_type: &SQLiteType) -> TokenStream {
    match column_type {
        SQLiteType::Integer => quote!(drizzle::sqlite::types::Integer),
        SQLiteType::Real => quote!(drizzle::sqlite::types::Real),
        SQLiteType::Blob => quote!(drizzle::sqlite::types::Blob),
        SQLiteType::Text => {
            let types = core_paths::types();
            quote!(#types::Text)
        }
        SQLiteType::Numeric | SQLiteType::Any => {
            let types = core_paths::types();
            quote!(#types::Any)
        }
    }
}

#[cfg(feature = "sqlite")]
pub fn sqlite_column_type_is_numeric(column_type: &SQLiteType) -> bool {
    matches!(
        column_type,
        SQLiteType::Integer | SQLiteType::Real | SQLiteType::Numeric
    )
}

#[cfg(feature = "postgres")]
pub fn postgres_column_type_to_sql_type(column_type: &PostgreSQLType) -> TokenStream {
    let types = core_paths::types();

    match column_type {
        PostgreSQLType::Smallint | PostgreSQLType::Smallserial => {
            quote!(drizzle::postgres::types::Int2)
        }
        PostgreSQLType::Integer | PostgreSQLType::Serial => {
            quote!(drizzle::postgres::types::Int4)
        }
        PostgreSQLType::Bigint | PostgreSQLType::Bigserial => {
            quote!(drizzle::postgres::types::Int8)
        }
        PostgreSQLType::Real => quote!(drizzle::postgres::types::Float4),
        PostgreSQLType::DoublePrecision => quote!(drizzle::postgres::types::Float8),
        PostgreSQLType::Text | PostgreSQLType::Varchar | PostgreSQLType::Char => {
            quote!(drizzle::postgres::types::Varchar)
        }
        PostgreSQLType::Boolean => quote!(drizzle::postgres::types::Boolean),
        PostgreSQLType::Bytea => quote!(drizzle::postgres::types::Bytea),
        PostgreSQLType::Timestamptz => quote!(drizzle::postgres::types::Timestamptz),
        PostgreSQLType::Timestamp => quote!(#types::Timestamp),
        PostgreSQLType::Date => quote!(#types::Date),
        PostgreSQLType::Time | PostgreSQLType::Timetz => quote!(#types::Time),
        PostgreSQLType::Numeric => quote!(#types::Any),
        #[cfg(feature = "uuid")]
        PostgreSQLType::Uuid => quote!(#types::Uuid),
        #[cfg(feature = "serde")]
        PostgreSQLType::Json => quote!(#types::Json),
        #[cfg(feature = "serde")]
        PostgreSQLType::Jsonb => quote!(#types::Jsonb),
        #[cfg(feature = "chrono")]
        PostgreSQLType::Interval => quote!(#types::Any),
        #[cfg(feature = "cidr")]
        PostgreSQLType::Inet
        | PostgreSQLType::Cidr
        | PostgreSQLType::MacAddr
        | PostgreSQLType::MacAddr8 => quote!(#types::Any),
        #[cfg(feature = "geo-types")]
        PostgreSQLType::Point
        | PostgreSQLType::Line
        | PostgreSQLType::Lseg
        | PostgreSQLType::Box
        | PostgreSQLType::Path
        | PostgreSQLType::Polygon
        | PostgreSQLType::Circle => quote!(#types::Any),
        #[cfg(feature = "bit-vec")]
        PostgreSQLType::Bit | PostgreSQLType::Varbit => quote!(#types::Any),
        PostgreSQLType::Enum(_) => quote!(#types::Any),
    }
}

#[cfg(feature = "postgres")]
pub fn postgres_column_type_is_numeric(column_type: &PostgreSQLType) -> bool {
    matches!(
        column_type,
        PostgreSQLType::Smallint
            | PostgreSQLType::Integer
            | PostgreSQLType::Bigint
            | PostgreSQLType::Smallserial
            | PostgreSQLType::Serial
            | PostgreSQLType::Bigserial
            | PostgreSQLType::Real
            | PostgreSQLType::DoublePrecision
            | PostgreSQLType::Numeric
    )
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
