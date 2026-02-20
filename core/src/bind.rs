use crate::dialect::{PostgresDialect, SQLiteDialect};
use crate::traits::SQLParam;
use crate::types::{Assignable, DataType};

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

impl ValueTypeForDialect<SQLiteDialect> for String {
    type SQLType = drizzle_types::sqlite::types::Text;
}

impl ValueTypeForDialect<SQLiteDialect> for &[u8] {
    type SQLType = drizzle_types::sqlite::types::Blob;
}

impl ValueTypeForDialect<SQLiteDialect> for Vec<u8> {
    type SQLType = drizzle_types::sqlite::types::Blob;
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

impl ValueTypeForDialect<PostgresDialect> for String {
    type SQLType = drizzle_types::postgres::types::Text;
}

impl ValueTypeForDialect<PostgresDialect> for &[u8] {
    type SQLType = drizzle_types::postgres::types::Bytea;
}

impl ValueTypeForDialect<PostgresDialect> for Vec<u8> {
    type SQLType = drizzle_types::postgres::types::Bytea;
}
