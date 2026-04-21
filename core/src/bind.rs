use crate::dialect::{PostgresDialect, SQLiteDialect};
use crate::traits::SQLParam;
use crate::types::{Assignable, DataType};

#[cfg(any(feature = "alloc", feature = "std"))]
use crate::prelude::{Box, Cow, String, Vec};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{rc::Rc, sync::Arc};
#[cfg(feature = "std")]
use std::{rc::Rc, sync::Arc};

/// Maps a Rust value type to its SQL marker for a specific dialect.
pub trait ValueTypeForDialect<D> {
    type SQLType: DataType;
}

/// Converts a Rust value into a dialect value while checking SQL marker assignment.
pub trait BindValue<'a, V: SQLParam, Expected: DataType>: Sized {
    fn into_bind_value(self) -> V;
}

/// Converts an optional Rust value into a nullable dialect value.
pub trait NullableBindValue<'a, V: SQLParam, Expected: DataType>: Sized {
    fn into_nullable_bind_value(self) -> V;
}

impl<V, Expected, T> BindValue<'_, V, Expected> for T
where
    V: SQLParam + From<T>,
    Expected: DataType + Assignable<<T as ValueTypeForDialect<V::DialectMarker>>::SQLType>,
    T: ValueTypeForDialect<V::DialectMarker>,
{
    fn into_bind_value(self) -> V {
        V::from(self)
    }
}

impl<V, Expected, T> NullableBindValue<'_, V, Expected> for Option<T>
where
    V: SQLParam + From<Self>,
    Expected: DataType + Assignable<<T as ValueTypeForDialect<V::DialectMarker>>::SQLType>,
    T: ValueTypeForDialect<V::DialectMarker>,
{
    fn into_nullable_bind_value(self) -> V {
        V::from(self)
    }
}

// =============================================================================
// ValueTypeForDialect impl generator
// =============================================================================

/// Declare `ValueTypeForDialect<$dialect>::SQLType = $sql` for one or more types.
macro_rules! impl_value_type {
    ($dialect:ty, $sql:ty => $($ty:ty),+ $(,)?) => {
        $(
            impl ValueTypeForDialect<$dialect> for $ty {
                type SQLType = $sql;
            }
        )+
    };
}

/// Same as `impl_value_type!`, but for types generic over `const N: usize`.
///
/// Only referenced under `arrayvec` / `smallvec-types` feature gates, so the
/// macro definition itself is cfg-gated to match — no broad `#[allow(unused_macros)]`.
#[cfg(any(feature = "arrayvec", feature = "smallvec-types"))]
macro_rules! impl_value_type_const_n {
    ($dialect:ty, $sql:ty => $($ty:ty),+ $(,)?) => {
        $(
            impl<const N: usize> ValueTypeForDialect<$dialect> for $ty {
                type SQLType = $sql;
            }
        )+
    };
}

// =============================================================================
// SQLite mappings
// =============================================================================

use drizzle_types::sqlite::types as sqlite_ty;

impl_value_type!(SQLiteDialect, sqlite_ty::Integer =>
    i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool);

impl_value_type!(SQLiteDialect, sqlite_ty::Real => f32, f64);

impl_value_type!(SQLiteDialect, sqlite_ty::Text => &str);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(SQLiteDialect, sqlite_ty::Text =>
    Cow<'_, str>,
    String,
    Box<String>,
    Rc<String>,
    Arc<String>,
    Box<str>,
    Rc<str>,
    Arc<str>,
);

impl_value_type!(SQLiteDialect, sqlite_ty::Text => compact_str::CompactString);

#[cfg(feature = "arrayvec")]
impl_value_type_const_n!(SQLiteDialect, sqlite_ty::Text => arrayvec::ArrayString<N>);

impl_value_type!(SQLiteDialect, sqlite_ty::Blob => &[u8]);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(SQLiteDialect, sqlite_ty::Blob =>
    Cow<'_, [u8]>,
    Vec<u8>,
    Box<Vec<u8>>,
    Rc<Vec<u8>>,
    Arc<Vec<u8>>,
);

#[cfg(feature = "arrayvec")]
impl_value_type_const_n!(SQLiteDialect, sqlite_ty::Blob => arrayvec::ArrayVec<u8, N>);

#[cfg(feature = "bytes")]
impl_value_type!(SQLiteDialect, sqlite_ty::Blob => bytes::Bytes, bytes::BytesMut);

#[cfg(feature = "smallvec-types")]
impl_value_type_const_n!(SQLiteDialect, sqlite_ty::Blob => smallvec::SmallVec<[u8; N]>);

#[cfg(feature = "uuid")]
impl_value_type!(SQLiteDialect, sqlite_ty::Blob => uuid::Uuid, &uuid::Uuid);

#[cfg(feature = "chrono")]
impl_value_type!(SQLiteDialect, sqlite_ty::Text =>
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::FixedOffset>,
    chrono::DateTime<chrono::Utc>,
    chrono::Duration,
);

#[cfg(feature = "time")]
impl_value_type!(SQLiteDialect, sqlite_ty::Text =>
    time::Date,
    time::Time,
    time::PrimitiveDateTime,
    time::OffsetDateTime,
    time::Duration,
);

#[cfg(feature = "rust-decimal")]
impl_value_type!(SQLiteDialect, sqlite_ty::Text =>
    rust_decimal::Decimal,
    &rust_decimal::Decimal,
);

#[cfg(feature = "serde")]
impl_value_type!(SQLiteDialect, sqlite_ty::Text => serde_json::Value);

// =============================================================================
// Postgres mappings
// =============================================================================

use drizzle_types::postgres::types as pg_ty;

impl_value_type!(PostgresDialect, pg_ty::Int2 => i8, i16, u8);
impl_value_type!(PostgresDialect, pg_ty::Int4 => i32, u16);
impl_value_type!(PostgresDialect, pg_ty::Int8 => i64, u32, u64, isize, usize);
impl_value_type!(PostgresDialect, pg_ty::Float4 => f32);
impl_value_type!(PostgresDialect, pg_ty::Float8 => f64);
impl_value_type!(PostgresDialect, pg_ty::Boolean => bool);

impl_value_type!(PostgresDialect, pg_ty::Text => &str);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, pg_ty::Text =>
    Cow<'_, str>,
    String,
    Box<String>,
    Rc<String>,
    Arc<String>,
    Box<str>,
    Rc<str>,
    Arc<str>,
);

#[cfg(feature = "compact-str")]
impl_value_type!(PostgresDialect, pg_ty::Text => compact_str::CompactString);

#[cfg(feature = "arrayvec")]
impl_value_type_const_n!(PostgresDialect, pg_ty::Text => arrayvec::ArrayString<N>);

impl_value_type!(PostgresDialect, pg_ty::Bytea => &[u8]);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, pg_ty::Bytea =>
    Cow<'_, [u8]>,
    Vec<u8>,
    Box<Vec<u8>>,
    Rc<Vec<u8>>,
    Arc<Vec<u8>>,
);

#[cfg(feature = "arrayvec")]
impl_value_type_const_n!(PostgresDialect, pg_ty::Bytea => arrayvec::ArrayVec<u8, N>);

#[cfg(feature = "bytes")]
impl_value_type!(PostgresDialect, pg_ty::Bytea => bytes::Bytes, bytes::BytesMut);

#[cfg(feature = "smallvec-types")]
impl_value_type_const_n!(PostgresDialect, pg_ty::Bytea => smallvec::SmallVec<[u8; N]>);

#[cfg(feature = "uuid")]
impl_value_type!(PostgresDialect, pg_ty::Uuid => uuid::Uuid, &uuid::Uuid);

#[cfg(feature = "rust-decimal")]
impl_value_type!(PostgresDialect, pg_ty::Numeric =>
    rust_decimal::Decimal,
    &rust_decimal::Decimal,
);

#[cfg(feature = "serde")]
impl_value_type!(PostgresDialect, pg_ty::Json => serde_json::Value);

#[cfg(feature = "chrono")]
impl_value_type!(PostgresDialect, pg_ty::Date => chrono::NaiveDate);
#[cfg(feature = "chrono")]
impl_value_type!(PostgresDialect, pg_ty::Time => chrono::NaiveTime);
#[cfg(feature = "chrono")]
impl_value_type!(PostgresDialect, pg_ty::Timestamp => chrono::NaiveDateTime);
#[cfg(feature = "chrono")]
impl_value_type!(PostgresDialect, pg_ty::Timestamptz =>
    chrono::DateTime<chrono::FixedOffset>,
    chrono::DateTime<chrono::Utc>,
);
#[cfg(feature = "chrono")]
impl_value_type!(PostgresDialect, pg_ty::Interval => chrono::Duration);

#[cfg(feature = "time")]
impl_value_type!(PostgresDialect, pg_ty::Date => time::Date);
#[cfg(feature = "time")]
impl_value_type!(PostgresDialect, pg_ty::Time => time::Time);
#[cfg(feature = "time")]
impl_value_type!(PostgresDialect, pg_ty::Timestamp => time::PrimitiveDateTime);
#[cfg(feature = "time")]
impl_value_type!(PostgresDialect, pg_ty::Timestamptz => time::OffsetDateTime);
#[cfg(feature = "time")]
impl_value_type!(PostgresDialect, pg_ty::Interval => time::Duration);

#[cfg(feature = "cidr")]
impl_value_type!(PostgresDialect, pg_ty::Inet => cidr::IpInet);
#[cfg(feature = "cidr")]
impl_value_type!(PostgresDialect, pg_ty::Cidr => cidr::IpCidr);
#[cfg(feature = "cidr")]
impl_value_type!(PostgresDialect, pg_ty::MacAddr => [u8; 6]);
#[cfg(feature = "cidr")]
impl_value_type!(PostgresDialect, pg_ty::MacAddr8 => [u8; 8]);

#[cfg(feature = "geo-types")]
impl_value_type!(PostgresDialect, pg_ty::Point => geo_types::Point<f64>);
#[cfg(feature = "geo-types")]
impl_value_type!(PostgresDialect, pg_ty::LineString => geo_types::LineString<f64>);
#[cfg(feature = "geo-types")]
impl_value_type!(PostgresDialect, pg_ty::Rect => geo_types::Rect<f64>);

#[cfg(feature = "bit-vec")]
impl_value_type!(PostgresDialect, pg_ty::BitString => bit_vec::BitVec);

// Postgres Vec<T> → Array<T>

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Text> =>
    Vec<String>, Vec<&str>);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Int2> => Vec<i16>);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Int4> => Vec<i32>);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Int8> => Vec<i64>);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Float4> => Vec<f32>);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Float8> => Vec<f64>);

#[cfg(any(feature = "alloc", feature = "std"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Boolean> => Vec<bool>);

#[cfg(all(any(feature = "alloc", feature = "std"), feature = "uuid"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Uuid> => Vec<uuid::Uuid>);

#[cfg(all(any(feature = "alloc", feature = "std"), feature = "chrono"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Date> => Vec<chrono::NaiveDate>);

#[cfg(all(any(feature = "alloc", feature = "std"), feature = "chrono"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Time> => Vec<chrono::NaiveTime>);

#[cfg(all(any(feature = "alloc", feature = "std"), feature = "chrono"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Timestamp> => Vec<chrono::NaiveDateTime>);

#[cfg(all(any(feature = "alloc", feature = "std"), feature = "chrono"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Timestamptz> => Vec<chrono::DateTime<chrono::Utc>>);

#[cfg(all(any(feature = "alloc", feature = "std"), feature = "rust-decimal"))]
impl_value_type!(PostgresDialect, drizzle_types::Array<pg_ty::Numeric> => Vec<rust_decimal::Decimal>);
