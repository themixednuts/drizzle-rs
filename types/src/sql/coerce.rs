use super::{
    Any, BigInt, Bool, Bytes, DataType, Date, Double, Float, Int, Json, Jsonb, SmallInt, Text,
    Time, Timestamp, TimestampTz, Uuid, VarChar,
};

#[diagnostic::on_unimplemented(
    message = "SQL type `{Self}` is not compatible with `{Rhs}`",
    label = "these SQL types cannot be compared or coerced",
    note = "compatible types include: integers with integers/floats, text with text/varchar, and any type with itself"
)]
pub trait Compatible<Rhs: DataType = Self>: DataType {}

impl<T: DataType> Compatible<T> for T {}

impl Compatible<Int> for SmallInt {}
impl Compatible<BigInt> for SmallInt {}
impl Compatible<SmallInt> for Int {}
impl Compatible<BigInt> for Int {}
impl Compatible<SmallInt> for BigInt {}
impl Compatible<Int> for BigInt {}

impl Compatible<Double> for Float {}
impl Compatible<Float> for Double {}

impl Compatible<Float> for SmallInt {}
impl Compatible<Double> for SmallInt {}
impl Compatible<Float> for Int {}
impl Compatible<Double> for Int {}
impl Compatible<Float> for BigInt {}
impl Compatible<Double> for BigInt {}

impl Compatible<SmallInt> for Float {}
impl Compatible<Int> for Float {}
impl Compatible<BigInt> for Float {}
impl Compatible<SmallInt> for Double {}
impl Compatible<Int> for Double {}
impl Compatible<BigInt> for Double {}

impl Compatible<VarChar> for Text {}
impl Compatible<Text> for VarChar {}

impl Compatible<Jsonb> for Json {}
impl Compatible<Json> for Jsonb {}

impl Compatible<TimestampTz> for Timestamp {}
impl Compatible<Timestamp> for TimestampTz {}

impl Compatible<crate::sqlite::types::Integer> for SmallInt {}
impl Compatible<crate::sqlite::types::Integer> for Int {}
impl Compatible<crate::sqlite::types::Integer> for BigInt {}
impl Compatible<SmallInt> for crate::sqlite::types::Integer {}
impl Compatible<Int> for crate::sqlite::types::Integer {}
impl Compatible<BigInt> for crate::sqlite::types::Integer {}

impl Compatible<crate::sqlite::types::Real> for Float {}
impl Compatible<crate::sqlite::types::Real> for Double {}
impl Compatible<Float> for crate::sqlite::types::Real {}
impl Compatible<Double> for crate::sqlite::types::Real {}

impl Compatible<crate::sqlite::types::Blob> for Bytes {}
impl Compatible<Bytes> for crate::sqlite::types::Blob {}
impl Compatible<crate::sqlite::types::Integer> for Any {}
impl Compatible<crate::sqlite::types::Real> for Any {}
impl Compatible<crate::sqlite::types::Blob> for Any {}
impl Compatible<Any> for crate::sqlite::types::Integer {}
impl Compatible<Any> for crate::sqlite::types::Real {}
impl Compatible<Any> for crate::sqlite::types::Blob {}

impl Compatible<crate::postgres::types::Int2> for SmallInt {}
impl Compatible<crate::postgres::types::Int4> for Int {}
impl Compatible<crate::postgres::types::Int8> for BigInt {}
impl Compatible<SmallInt> for crate::postgres::types::Int2 {}
impl Compatible<Int> for crate::postgres::types::Int4 {}
impl Compatible<BigInt> for crate::postgres::types::Int8 {}

impl Compatible<crate::postgres::types::Float4> for Float {}
impl Compatible<crate::postgres::types::Float8> for Double {}
impl Compatible<Float> for crate::postgres::types::Float4 {}
impl Compatible<Double> for crate::postgres::types::Float8 {}

impl Compatible<crate::postgres::types::Varchar> for Text {}
impl Compatible<crate::postgres::types::Varchar> for VarChar {}
impl Compatible<Text> for crate::postgres::types::Varchar {}
impl Compatible<VarChar> for crate::postgres::types::Varchar {}

impl Compatible<crate::postgres::types::Bytea> for Bytes {}
impl Compatible<Bytes> for crate::postgres::types::Bytea {}

impl Compatible<crate::postgres::types::Boolean> for Bool {}
impl Compatible<Bool> for crate::postgres::types::Boolean {}

impl Compatible<crate::postgres::types::Timestamptz> for TimestampTz {}
impl Compatible<TimestampTz> for crate::postgres::types::Timestamptz {}
impl Compatible<crate::postgres::types::Int2> for Any {}
impl Compatible<crate::postgres::types::Int4> for Any {}
impl Compatible<crate::postgres::types::Int8> for Any {}
impl Compatible<crate::postgres::types::Float4> for Any {}
impl Compatible<crate::postgres::types::Float8> for Any {}
impl Compatible<crate::postgres::types::Varchar> for Any {}
impl Compatible<crate::postgres::types::Bytea> for Any {}
impl Compatible<crate::postgres::types::Boolean> for Any {}
impl Compatible<crate::postgres::types::Timestamptz> for Any {}
impl Compatible<Any> for crate::postgres::types::Int2 {}
impl Compatible<Any> for crate::postgres::types::Int4 {}
impl Compatible<Any> for crate::postgres::types::Int8 {}
impl Compatible<Any> for crate::postgres::types::Float4 {}
impl Compatible<Any> for crate::postgres::types::Float8 {}
impl Compatible<Any> for crate::postgres::types::Varchar {}
impl Compatible<Any> for crate::postgres::types::Bytea {}
impl Compatible<Any> for crate::postgres::types::Boolean {}
impl Compatible<Any> for crate::postgres::types::Timestamptz {}

impl Compatible<Text> for Date {}
impl Compatible<VarChar> for Date {}
impl Compatible<Date> for Text {}
impl Compatible<Date> for VarChar {}

impl Compatible<Text> for Time {}
impl Compatible<VarChar> for Time {}
impl Compatible<Time> for Text {}
impl Compatible<Time> for VarChar {}

impl Compatible<Text> for Timestamp {}
impl Compatible<VarChar> for Timestamp {}
impl Compatible<Timestamp> for Text {}
impl Compatible<Timestamp> for VarChar {}

impl Compatible<Text> for TimestampTz {}
impl Compatible<VarChar> for TimestampTz {}
impl Compatible<TimestampTz> for Text {}
impl Compatible<TimestampTz> for VarChar {}

impl Compatible<SmallInt> for Any {}
impl Compatible<Int> for Any {}
impl Compatible<BigInt> for Any {}
impl Compatible<Float> for Any {}
impl Compatible<Double> for Any {}
impl Compatible<Text> for Any {}
impl Compatible<VarChar> for Any {}
impl Compatible<Bool> for Any {}
impl Compatible<Bytes> for Any {}
impl Compatible<Date> for Any {}
impl Compatible<Time> for Any {}
impl Compatible<Timestamp> for Any {}
impl Compatible<TimestampTz> for Any {}
impl Compatible<Uuid> for Any {}
impl Compatible<Json> for Any {}
impl Compatible<Jsonb> for Any {}

impl Compatible<Any> for SmallInt {}
impl Compatible<Any> for Int {}
impl Compatible<Any> for BigInt {}
impl Compatible<Any> for Float {}
impl Compatible<Any> for Double {}
impl Compatible<Any> for Text {}
impl Compatible<Any> for VarChar {}
impl Compatible<Any> for Bool {}
impl Compatible<Any> for Bytes {}
impl Compatible<Any> for Date {}
impl Compatible<Any> for Time {}
impl Compatible<Any> for Timestamp {}
impl Compatible<Any> for TimestampTz {}
impl Compatible<Any> for Uuid {}
impl Compatible<Any> for Json {}
impl Compatible<Any> for Jsonb {}

impl Compatible<Text> for Uuid {}
impl Compatible<VarChar> for Uuid {}
impl Compatible<Uuid> for Text {}
impl Compatible<Uuid> for VarChar {}
