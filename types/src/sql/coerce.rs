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
