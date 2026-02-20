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
        SQLiteType::Text => quote!(drizzle::sqlite::types::Text),
        SQLiteType::Real => quote!(drizzle::sqlite::types::Real),
        SQLiteType::Blob => quote!(drizzle::sqlite::types::Blob),
        SQLiteType::Numeric => quote!(drizzle::sqlite::types::Numeric),
        SQLiteType::Any => quote!(drizzle::sqlite::types::Any),
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
        PostgreSQLType::Text => quote!(drizzle::postgres::types::Text),
        PostgreSQLType::Varchar => quote!(drizzle::postgres::types::Varchar),
        PostgreSQLType::Char => quote!(drizzle::postgres::types::Char),
        PostgreSQLType::Boolean => quote!(drizzle::postgres::types::Boolean),
        PostgreSQLType::Bytea => quote!(drizzle::postgres::types::Bytea),
        PostgreSQLType::Timestamptz => quote!(drizzle::postgres::types::Timestamptz),
        PostgreSQLType::Timestamp => quote!(drizzle::postgres::types::Timestamp),
        PostgreSQLType::Date => quote!(drizzle::postgres::types::Date),
        PostgreSQLType::Time => quote!(drizzle::postgres::types::Time),
        PostgreSQLType::Timetz => quote!(drizzle::postgres::types::Timetz),
        PostgreSQLType::Numeric => quote!(drizzle::postgres::types::Numeric),
        #[cfg(feature = "uuid")]
        PostgreSQLType::Uuid => quote!(drizzle::postgres::types::Uuid),
        #[cfg(feature = "serde")]
        PostgreSQLType::Json => quote!(drizzle::postgres::types::Json),
        #[cfg(feature = "serde")]
        PostgreSQLType::Jsonb => quote!(drizzle::postgres::types::Jsonb),
        #[cfg(feature = "chrono")]
        PostgreSQLType::Interval => quote!(drizzle::postgres::types::Interval),
        #[cfg(feature = "cidr")]
        PostgreSQLType::Inet => quote!(drizzle::postgres::types::Inet),
        #[cfg(feature = "cidr")]
        PostgreSQLType::Cidr => quote!(drizzle::postgres::types::Cidr),
        #[cfg(feature = "cidr")]
        PostgreSQLType::MacAddr => quote!(drizzle::postgres::types::MacAddr),
        #[cfg(feature = "cidr")]
        PostgreSQLType::MacAddr8 => quote!(drizzle::postgres::types::MacAddr8),
        #[cfg(feature = "geo-types")]
        PostgreSQLType::Point => {
            quote!(drizzle::postgres::types::Point)
        }
        #[cfg(feature = "geo-types")]
        PostgreSQLType::Line => quote!(drizzle::postgres::types::Line),
        #[cfg(feature = "geo-types")]
        PostgreSQLType::Lseg => quote!(drizzle::postgres::types::LineSegment),
        #[cfg(feature = "geo-types")]
        PostgreSQLType::Box => quote!(drizzle::postgres::types::Rect),
        #[cfg(feature = "geo-types")]
        PostgreSQLType::Path => quote!(drizzle::postgres::types::LineString),
        #[cfg(feature = "geo-types")]
        PostgreSQLType::Polygon => quote!(drizzle::postgres::types::Polygon),
        #[cfg(feature = "geo-types")]
        PostgreSQLType::Circle => quote!(drizzle::postgres::types::Circle),
        #[cfg(feature = "bit-vec")]
        PostgreSQLType::Bit | PostgreSQLType::Varbit => {
            quote!(drizzle::postgres::types::BitString)
        }
        PostgreSQLType::Enum(_) => quote!(drizzle::postgres::types::Enum),
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
