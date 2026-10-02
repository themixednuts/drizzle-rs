//! Expr implementations for Rust primitive types.
//!
//! These implementations allow using Rust literals directly in type-safe
//! SQL expressions.

use crate::bind::ValueTypeForDialect;
use crate::dialect::DialectTypes;
use crate::prelude::*;
use crate::sql::SQL;
use crate::traits::{SQLBytes, SQLParam};

use super::{Expr, ExprSources, NonNull, Null, Nullability, Scalar};

// =============================================================================
// Integer Types
// =============================================================================

impl<'a, V> Expr<'a, V> for i8
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i16
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i32
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for i64
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for isize
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// Unsigned Integer Types
// =============================================================================

impl<'a, V> Expr<'a, V> for u8
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u16
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u32
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for u64
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for usize
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// Floating-Point Types
// =============================================================================

impl<'a, V> Expr<'a, V> for f32
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for f64
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// Boolean Type
// =============================================================================

impl<'a, V> Expr<'a, V> for bool
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// String Types
// =============================================================================

impl<'a, V> Expr<'a, V> for &'a str
where
    V: SQLParam + 'a + From<&'a str> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V> Expr<'a, V> for String
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

#[cfg(feature = "compact-str")]
impl<'a, V> Expr<'a, V> for compact_str::CompactString
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

#[cfg(feature = "bytes")]
impl<'a, V> Expr<'a, V> for bytes::Bytes
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

#[cfg(feature = "bytes")]
impl<'a, V> Expr<'a, V> for bytes::BytesMut
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

#[cfg(feature = "arrayvec")]
impl<'a, V, const N: usize> Expr<'a, V> for arrayvec::ArrayString<N>
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

#[cfg(feature = "arrayvec")]
impl<'a, V, const N: usize> Expr<'a, V> for arrayvec::ArrayVec<u8, N>
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

#[cfg(feature = "smallvec-types")]
impl<'a, V, const N: usize> Expr<'a, V> for smallvec::SmallVec<[u8; N]>
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V, const N: usize> Expr<'a, V> for [char; N]
where
    V: SQLParam + 'a + From<Self> + From<char> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(*self))
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
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

impl<T: ExprSources> ExprSources for Option<T> {
    type Sources = T::Sources;
}

impl<'a, V, T> Expr<'a, V> for Option<T>
where
    V: SQLParam + 'a,
    T: Expr<'a, V>,
    T::Nullable: Nullability,
{
    type SQLType = T::SQLType;
    type Nullable = Null;
    type Aggregate = T::Aggregate;

    /// A missing optional condition contributes nothing to a condition list.
    fn to_condition_sql(&self) -> Option<SQL<'a, V>> {
        self.as_ref().map(<T as Expr<'a, V>>::to_expr_sql)
    }

    /// A missing optional condition contributes nothing to a condition list.
    fn into_condition_sql(self) -> Option<SQL<'a, V>> {
        self.map(<T as Expr<'a, V>>::into_expr_sql)
    }
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

    fn to_condition_sql(&self) -> Option<SQL<'a, V>> {
        (**self).to_condition_sql()
    }

    fn into_condition_sql(self) -> Option<SQL<'a, V>> {
        (*self).to_condition_sql()
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

#[cfg(feature = "rust-decimal")]
impl<'a, V> Expr<'a, V> for rust_decimal::Decimal
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
    Self: ValueTypeForDialect<V::DialectMarker>,
{
    type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

// =============================================================================
// Date and Time Values (Feature-Gated)
// =============================================================================

/// Lets a date or time value stand where an expression goes (`eq(col, value)`,
/// `gt(col, value)`), typed as the SQL type its dialect stores it as.
#[cfg(any(feature = "chrono", feature = "time", feature = "jiff"))]
macro_rules! impl_value_expr {
    ($($ty:ty),+ $(,)?) => {$(
        impl<'a, V> Expr<'a, V> for $ty
        where
            V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
            Self: ValueTypeForDialect<V::DialectMarker>,
        {
            type SQLType = <Self as ValueTypeForDialect<V::DialectMarker>>::SQLType;
            type Nullable = NonNull;
            type Aggregate = Scalar;
        }

        impl ExprSources for $ty {
            type Sources = ();
        }
    )+};
}

#[cfg(feature = "chrono")]
impl_value_expr!(
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::FixedOffset>,
    chrono::Duration,
);

#[cfg(feature = "time")]
impl_value_expr!(
    time::Date,
    time::Time,
    time::PrimitiveDateTime,
    time::OffsetDateTime,
    time::Duration,
);

#[cfg(feature = "jiff")]
impl_value_expr!(
    jiff::civil::Date,
    jiff::civil::Time,
    jiff::civil::DateTime,
    jiff::Timestamp,
);

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

// Literals and raw SQL read no source.
impl ExprSources for i8 {
    type Sources = ();
}

impl ExprSources for i16 {
    type Sources = ();
}

impl ExprSources for i32 {
    type Sources = ();
}

impl ExprSources for i64 {
    type Sources = ();
}

impl ExprSources for isize {
    type Sources = ();
}

impl ExprSources for u8 {
    type Sources = ();
}

impl ExprSources for u16 {
    type Sources = ();
}

impl ExprSources for u32 {
    type Sources = ();
}

impl ExprSources for u64 {
    type Sources = ();
}

impl ExprSources for usize {
    type Sources = ();
}

impl ExprSources for f32 {
    type Sources = ();
}

impl ExprSources for f64 {
    type Sources = ();
}

impl ExprSources for bool {
    type Sources = ();
}

impl ExprSources for &str {
    type Sources = ();
}

impl ExprSources for String {
    type Sources = ();
}

#[cfg(feature = "compact-str")]
impl ExprSources for compact_str::CompactString {
    type Sources = ();
}

#[cfg(feature = "bytes")]
impl ExprSources for bytes::Bytes {
    type Sources = ();
}

#[cfg(feature = "bytes")]
impl ExprSources for bytes::BytesMut {
    type Sources = ();
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> ExprSources for arrayvec::ArrayString<N> {
    type Sources = ();
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> ExprSources for arrayvec::ArrayVec<u8, N> {
    type Sources = ();
}

#[cfg(feature = "smallvec-types")]
impl<const N: usize> ExprSources for smallvec::SmallVec<[u8; N]> {
    type Sources = ();
}

impl<const N: usize> ExprSources for [char; N] {
    type Sources = ();
}

impl ExprSources for &[u8] {
    type Sources = ();
}

impl<const N: usize> ExprSources for [u8; N] {
    type Sources = ();
}

impl ExprSources for Vec<u8> {
    type Sources = ();
}

impl ExprSources for Cow<'_, [u8]> {
    type Sources = ();
}

impl ExprSources for SQLBytes<'_> {
    type Sources = ();
}

#[cfg(feature = "uuid")]
impl ExprSources for uuid::Uuid {
    type Sources = ();
}

#[cfg(feature = "rust-decimal")]
impl ExprSources for rust_decimal::Decimal {
    type Sources = ();
}

impl<V: SQLParam> ExprSources for crate::sql::SQL<'_, V> {
    type Sources = ();
}
