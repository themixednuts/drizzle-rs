//! Expr implementations for Rust primitive types.
//!
//! These implementations allow using Rust literals directly in type-safe
//! SQL expressions.

use crate::dialect::DialectTypes;
use crate::prelude::*;
use crate::sql::SQL;
use crate::traits::{SQLBytes, SQLParam};

use super::{Expr, NonNull, Null, Nullability, Scalar};

// =============================================================================
// Integer Types
// =============================================================================

impl<'a, V> Expr<'a, V> for i8
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::SmallInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i16
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::SmallInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i32
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Int;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i64
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::BigInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for isize
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::BigInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// Unsigned Integer Types
// =============================================================================

impl<'a, V> Expr<'a, V> for u8
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::SmallInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u16
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Int;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u32
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::BigInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u64
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::BigInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for usize
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::BigInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// Floating-Point Types
// =============================================================================

impl<'a, V> Expr<'a, V> for f32
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Float;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for f64
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Double;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// Boolean Type
// =============================================================================

impl<'a, V> Expr<'a, V> for bool
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Bool;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// String Types
// =============================================================================

impl<'a, V> Expr<'a, V> for &'a str
where
    V: SQLParam + 'a + From<&'a str> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Text;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for String
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Text;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// Binary Types
// =============================================================================

impl<'a, V> Expr<'a, V> for &'a [u8]
where
    V: SQLParam + 'a + From<&'a [u8]> + From<Vec<u8>> + From<u8> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Bytes;
    type Nullable = NonNull;
    type Aggregate = Scalar;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        SQL::bytes(*self)
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        SQL::bytes(self)
    }
}

impl<'a, V, const N: usize> Expr<'a, V> for [u8; N]
where
    V: SQLParam + 'a + From<&'a [u8]> + From<Vec<u8>> + From<u8> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Bytes;
    type Nullable = NonNull;
    type Aggregate = Scalar;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        SQL::bytes(self.to_vec())
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        SQL::bytes(self.to_vec())
    }
}

impl<'a, V> Expr<'a, V> for Vec<u8>
where
    V: SQLParam + 'a + From<&'a [u8]> + From<Vec<u8>> + From<u8> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Bytes;
    type Nullable = NonNull;
    type Aggregate = Scalar;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        SQL::bytes(self.clone())
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        SQL::bytes(self)
    }
}

impl<'a, V> Expr<'a, V> for Cow<'a, [u8]>
where
    V: SQLParam + 'a + From<&'a [u8]> + From<Vec<u8>> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Bytes;
    type Nullable = NonNull;
    type Aggregate = Scalar;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        match self {
            Cow::Borrowed(value) => SQL::bytes(*value),
            Cow::Owned(value) => SQL::bytes(value.clone()),
        }
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        SQL::bytes(self)
    }
}

impl<'a, V> Expr<'a, V> for SQLBytes<'a>
where
    V: SQLParam + 'a + From<&'a [u8]> + From<Vec<u8>> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Bytes;
    type Nullable = NonNull;
    type Aggregate = Scalar;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        match &self.0 {
            Cow::Borrowed(value) => SQL::bytes(*value),
            Cow::Owned(value) => SQL::bytes(value.clone()),
        }
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        SQL::bytes(self.0)
    }
}

// =============================================================================
// Option<T> - Makes Any Expression Nullable
// =============================================================================

impl<'a, V, T> Expr<'a, V> for Option<T>
where
    V: SQLParam + 'a,
    T: Expr<'a, V>,
    T::Nullable: Nullability,
{
    type SQLType = T::SQLType;
    type Nullable = Null;
    type Aggregate = T::Aggregate;
}

// =============================================================================
// Reference Types - Delegate to Inner
// =============================================================================

impl<'a, V, T> Expr<'a, V> for &T
where
    V: SQLParam + 'a,
    T: Expr<'a, V>,
    T::Nullable: Nullability,
{
    type SQLType = T::SQLType;
    type Nullable = T::Nullable;
    type Aggregate = T::Aggregate;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        (**self).to_expr_sql()
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        (*self).to_expr_sql()
    }
}

// =============================================================================
// UUID (Feature-Gated)
// =============================================================================

#[cfg(feature = "uuid")]
impl<'a, V> Expr<'a, V> for uuid::Uuid
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Uuid;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// SQL Type - Backward Compatibility
// Allows untyped columns (which return SQL) to work with typed functions.
// =============================================================================

impl<'a, V> Expr<'a, V> for crate::sql::SQL<'a, V>
where
    V: SQLParam + 'a,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Any;
    type Nullable = Null;
    type Aggregate = Scalar;
}
