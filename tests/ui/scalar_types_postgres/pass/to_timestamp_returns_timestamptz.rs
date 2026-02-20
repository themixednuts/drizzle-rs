use drizzle::core::expr::{raw_non_null, to_timestamp, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

fn main() {
    let _: SQLExpr<'static, PostgresValue, drizzle::postgres::types::Timestamptz, NonNull, Scalar> =
        to_timestamp(raw_non_null::<PostgresValue, drizzle::postgres::types::Int8>("1700000000"));
}
