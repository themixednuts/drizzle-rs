use drizzle::core::expr::{raw_non_null, to_timestamp, NonNull, SQLExpr, Scalar};
use drizzle::core::types::{BigInt, TimestampTz};
use drizzle::postgres::prelude::*;

fn main() {
    let _: SQLExpr<'static, PostgresValue, TimestampTz, NonNull, Scalar> =
        to_timestamp(raw_non_null::<PostgresValue, BigInt>("1700000000"));
}
