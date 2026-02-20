//! Expr implementations for Rust primitive types.
//!
//! These implementations allow using Rust literals directly in type-safe
//! SQL expressions.

use crate::dialect::DialectTypes;
use crate::prelude::*;
use crate::traits::SQLParam;

use super::{Expr, NonNull, Null, Nullability, Scalar};

// =============================================================================
// Integer Types
// =============================================================================

impl<'a, V> Expr<'a, V> for i8
where
    V: SQLParam + 'a + From<i8>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::SmallInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i16
where
    V: SQLParam + 'a + From<i16>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::SmallInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i32
where
    V: SQLParam + 'a + From<i32>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Int;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i64
where
    V: SQLParam + 'a + From<i64>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::BigInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for isize
where
    V: SQLParam + 'a + From<isize>,
    V: Into<Cow<'a, V>>,
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
    V: SQLParam + 'a + From<u8>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::SmallInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u16
where
    V: SQLParam + 'a + From<u16>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Int;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u32
where
    V: SQLParam + 'a + From<u32>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::BigInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u64
where
    V: SQLParam + 'a + From<u64>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::BigInt;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for usize
where
    V: SQLParam + 'a + From<usize>,
    V: Into<Cow<'a, V>>,
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
    V: SQLParam + 'a + From<f32>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Float;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for f64
where
    V: SQLParam + 'a + From<f64>,
    V: Into<Cow<'a, V>>,
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
    V: SQLParam + 'a + From<bool>,
    V: Into<Cow<'a, V>>,
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
    V: SQLParam + 'a + From<&'a str>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Text;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for String
where
    V: SQLParam + 'a + From<String>,
    V: Into<Cow<'a, V>>,
{
    type SQLType = <V::DialectMarker as DialectTypes>::Text;
    type Nullable = NonNull;
    type Aggregate = Scalar;
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
}

// =============================================================================
// UUID (Feature-Gated)
// =============================================================================

#[cfg(feature = "uuid")]
impl<'a, V> Expr<'a, V> for uuid::Uuid
where
    V: SQLParam + 'a + From<uuid::Uuid>,
    V: Into<Cow<'a, V>>,
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
