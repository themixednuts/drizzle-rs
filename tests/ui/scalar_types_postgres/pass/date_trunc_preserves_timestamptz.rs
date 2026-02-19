use drizzle::core::expr::{date_trunc, raw_non_null, NonNull, SQLExpr, Scalar};
use drizzle::core::types::TimestampTz;
use drizzle::postgres::prelude::*;

fn main() {
    let _: SQLExpr<'static, PostgresValue, TimestampTz, NonNull, Scalar> =
        date_trunc("day", raw_non_null::<PostgresValue, TimestampTz>("NOW()"));
}
