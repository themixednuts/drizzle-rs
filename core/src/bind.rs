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

impl<'a, V, Expected, T> BindValue<'a, V, Expected> for T
where
    V: SQLParam + From<T>,
    Expected: DataType + Assignable<<T as ValueTypeForDialect<V::DialectMarker>>::SQLType>,
    T: ValueTypeForDialect<V::DialectMarker>,
{
    fn into_bind_value(self) -> V {
        V::from(self)
    }
}

impl<'a, V, Expected, T> NullableBindValue<'a, V, Expected> for Option<T>
where
    V: SQLParam + From<Option<T>>,
    Expected: DataType + Assignable<<T as ValueTypeForDialect<V::DialectMarker>>::SQLType>,
    T: ValueTypeForDialect<V::DialectMarker>,
{
    fn into_nullable_bind_value(self) -> V {
        V::from(self)
    }
}

macro_rules! impl_sqlite_integer {
    ($($t:ty),+ $(,)?) => {
        $(
            impl ValueTypeForDialect<SQLiteDialect> for $t {
                type SQLType = drizzle_types::sqlite::types::Integer;
            }
        )+
    };
}

impl_sqlite_integer!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool);

impl ValueTypeForDialect<SQLiteDialect> for f32 {
    type SQLType = drizzle_types::sqlite::types::Real;
}

impl ValueTypeForDialect<SQLiteDialect> for f64 {
    type SQLType = drizzle_types::sqlite::types::Real;
}

impl ValueTypeForDialect<SQLiteDialect> for &str {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<'a> ValueTypeForDialect<SQLiteDialect> for Cow<'a, str> {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for String {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Box<String> {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Rc<String> {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Arc<String> {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Box<str> {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Rc<str> {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Arc<str> {
    type SQLType = drizzle_types::sqlite::types::Text;
}

impl ValueTypeForDialect<SQLiteDialect> for compact_str::CompactString {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> ValueTypeForDialect<SQLiteDialect> for arrayvec::ArrayString<N> {
    type SQLType = drizzle_types::sqlite::types::Text;
}

impl ValueTypeForDialect<SQLiteDialect> for &[u8] {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<'a> ValueTypeForDialect<SQLiteDialect> for Cow<'a, [u8]> {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Vec<u8> {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Box<Vec<u8>> {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Rc<Vec<u8>> {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<SQLiteDialect> for Arc<Vec<u8>> {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> ValueTypeForDialect<SQLiteDialect> for arrayvec::ArrayVec<u8, N> {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(feature = "bytes")]
impl ValueTypeForDialect<SQLiteDialect> for bytes::Bytes {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(feature = "bytes")]
impl ValueTypeForDialect<SQLiteDialect> for bytes::BytesMut {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(feature = "smallvec-types")]
impl<const N: usize> ValueTypeForDialect<SQLiteDialect> for smallvec::SmallVec<[u8; N]> {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(feature = "uuid")]
impl ValueTypeForDialect<SQLiteDialect> for uuid::Uuid {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

#[cfg(feature = "uuid")]
impl ValueTypeForDialect<SQLiteDialect> for &uuid::Uuid {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

impl ValueTypeForDialect<PostgresDialect> for i8 {
    type SQLType = drizzle_types::postgres::types::Int2;
}

impl ValueTypeForDialect<PostgresDialect> for i16 {
    type SQLType = drizzle_types::postgres::types::Int2;
}

impl ValueTypeForDialect<PostgresDialect> for i32 {
    type SQLType = drizzle_types::postgres::types::Int4;
}

impl ValueTypeForDialect<PostgresDialect> for i64 {
    type SQLType = drizzle_types::postgres::types::Int8;
}

impl ValueTypeForDialect<PostgresDialect> for u8 {
    type SQLType = drizzle_types::postgres::types::Int2;
}

impl ValueTypeForDialect<PostgresDialect> for u16 {
    type SQLType = drizzle_types::postgres::types::Int4;
}

impl ValueTypeForDialect<PostgresDialect> for u32 {
    type SQLType = drizzle_types::postgres::types::Int8;
}

impl ValueTypeForDialect<PostgresDialect> for u64 {
    type SQLType = drizzle_types::postgres::types::Int8;
}

impl ValueTypeForDialect<PostgresDialect> for isize {
    type SQLType = drizzle_types::postgres::types::Int8;
}

impl ValueTypeForDialect<PostgresDialect> for usize {
    type SQLType = drizzle_types::postgres::types::Int8;
}

impl ValueTypeForDialect<PostgresDialect> for f32 {
    type SQLType = drizzle_types::postgres::types::Float4;
}

impl ValueTypeForDialect<PostgresDialect> for f64 {
    type SQLType = drizzle_types::postgres::types::Float8;
}

impl ValueTypeForDialect<PostgresDialect> for bool {
    type SQLType = drizzle_types::postgres::types::Boolean;
}

impl ValueTypeForDialect<PostgresDialect> for &str {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<'a> ValueTypeForDialect<PostgresDialect> for Cow<'a, str> {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for String {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Box<String> {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Rc<String> {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Arc<String> {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Box<str> {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Rc<str> {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Arc<str> {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(feature = "compact-str")]
impl ValueTypeForDialect<PostgresDialect> for compact_str::CompactString {
    type SQLType = drizzle_types::postgres::types::Text;
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> ValueTypeForDialect<PostgresDialect> for arrayvec::ArrayString<N> {
    type SQLType = drizzle_types::postgres::types::Text;
}

impl ValueTypeForDialect<PostgresDialect> for &[u8] {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<'a> ValueTypeForDialect<PostgresDialect> for Cow<'a, [u8]> {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<u8> {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Box<Vec<u8>> {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Rc<Vec<u8>> {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Arc<Vec<u8>> {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> ValueTypeForDialect<PostgresDialect> for arrayvec::ArrayVec<u8, N> {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(feature = "bytes")]
impl ValueTypeForDialect<PostgresDialect> for bytes::Bytes {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(feature = "bytes")]
impl ValueTypeForDialect<PostgresDialect> for bytes::BytesMut {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(feature = "smallvec-types")]
impl<const N: usize> ValueTypeForDialect<PostgresDialect> for smallvec::SmallVec<[u8; N]> {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

#[cfg(feature = "uuid")]
impl ValueTypeForDialect<PostgresDialect> for uuid::Uuid {
    type SQLType = drizzle_types::postgres::types::Uuid;
}

#[cfg(feature = "uuid")]
impl ValueTypeForDialect<PostgresDialect> for &uuid::Uuid {
    type SQLType = drizzle_types::postgres::types::Uuid;
}

#[cfg(feature = "rust-decimal")]
impl ValueTypeForDialect<PostgresDialect> for rust_decimal::Decimal {
    type SQLType = drizzle_types::postgres::types::Numeric;
}

#[cfg(feature = "rust-decimal")]
impl ValueTypeForDialect<PostgresDialect> for &rust_decimal::Decimal {
    type SQLType = drizzle_types::postgres::types::Numeric;
}

#[cfg(feature = "serde")]
impl ValueTypeForDialect<SQLiteDialect> for serde_json::Value {
    type SQLType = drizzle_types::sqlite::types::Text;
}

#[cfg(feature = "serde")]
impl ValueTypeForDialect<PostgresDialect> for serde_json::Value {
    type SQLType = drizzle_types::postgres::types::Json;
}

#[cfg(feature = "chrono")]
impl ValueTypeForDialect<PostgresDialect> for chrono::NaiveDate {
    type SQLType = drizzle_types::postgres::types::Date;
}

#[cfg(feature = "chrono")]
impl ValueTypeForDialect<PostgresDialect> for chrono::NaiveTime {
    type SQLType = drizzle_types::postgres::types::Time;
}

#[cfg(feature = "chrono")]
impl ValueTypeForDialect<PostgresDialect> for chrono::NaiveDateTime {
    type SQLType = drizzle_types::postgres::types::Timestamp;
}

#[cfg(feature = "chrono")]
impl ValueTypeForDialect<PostgresDialect> for chrono::DateTime<chrono::FixedOffset> {
    type SQLType = drizzle_types::postgres::types::Timestamptz;
}

#[cfg(feature = "chrono")]
impl ValueTypeForDialect<PostgresDialect> for chrono::DateTime<chrono::Utc> {
    type SQLType = drizzle_types::postgres::types::Timestamptz;
}

#[cfg(feature = "chrono")]
impl ValueTypeForDialect<PostgresDialect> for chrono::Duration {
    type SQLType = drizzle_types::postgres::types::Interval;
}

#[cfg(feature = "time")]
impl ValueTypeForDialect<PostgresDialect> for time::Date {
    type SQLType = drizzle_types::postgres::types::Date;
}

#[cfg(feature = "time")]
impl ValueTypeForDialect<PostgresDialect> for time::Time {
    type SQLType = drizzle_types::postgres::types::Time;
}

#[cfg(feature = "time")]
impl ValueTypeForDialect<PostgresDialect> for time::PrimitiveDateTime {
    type SQLType = drizzle_types::postgres::types::Timestamp;
}

#[cfg(feature = "time")]
impl ValueTypeForDialect<PostgresDialect> for time::OffsetDateTime {
    type SQLType = drizzle_types::postgres::types::Timestamptz;
}

#[cfg(feature = "time")]
impl ValueTypeForDialect<PostgresDialect> for time::Duration {
    type SQLType = drizzle_types::postgres::types::Interval;
}

#[cfg(feature = "cidr")]
impl ValueTypeForDialect<PostgresDialect> for cidr::IpInet {
    type SQLType = drizzle_types::postgres::types::Inet;
}

#[cfg(feature = "cidr")]
impl ValueTypeForDialect<PostgresDialect> for cidr::IpCidr {
    type SQLType = drizzle_types::postgres::types::Cidr;
}

#[cfg(feature = "cidr")]
impl ValueTypeForDialect<PostgresDialect> for [u8; 6] {
    type SQLType = drizzle_types::postgres::types::MacAddr;
}

#[cfg(feature = "cidr")]
impl ValueTypeForDialect<PostgresDialect> for [u8; 8] {
    type SQLType = drizzle_types::postgres::types::MacAddr8;
}

#[cfg(feature = "geo-types")]
impl ValueTypeForDialect<PostgresDialect> for geo_types::Point<f64> {
    type SQLType = drizzle_types::postgres::types::Point;
}

#[cfg(feature = "geo-types")]
impl ValueTypeForDialect<PostgresDialect> for geo_types::LineString<f64> {
    type SQLType = drizzle_types::postgres::types::LineString;
}

#[cfg(feature = "geo-types")]
impl ValueTypeForDialect<PostgresDialect> for geo_types::Rect<f64> {
    type SQLType = drizzle_types::postgres::types::Rect;
}

#[cfg(feature = "bit-vec")]
impl ValueTypeForDialect<PostgresDialect> for bit_vec::BitVec {
    type SQLType = drizzle_types::postgres::types::BitString;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<String> {
    type SQLType = drizzle_types::Array<drizzle_types::postgres::types::Text>;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<&str> {
    type SQLType = drizzle_types::Array<drizzle_types::postgres::types::Text>;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<i16> {
    type SQLType = drizzle_types::Array<drizzle_types::postgres::types::Int2>;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<i32> {
    type SQLType = drizzle_types::Array<drizzle_types::postgres::types::Int4>;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<i64> {
    type SQLType = drizzle_types::Array<drizzle_types::postgres::types::Int8>;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<f32> {
    type SQLType = drizzle_types::Array<drizzle_types::postgres::types::Float4>;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<f64> {
    type SQLType = drizzle_types::Array<drizzle_types::postgres::types::Float8>;
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl ValueTypeForDialect<PostgresDialect> for Vec<bool> {
    type SQLType = drizzle_types::Array<drizzle_types::postgres::types::Boolean>;
}
