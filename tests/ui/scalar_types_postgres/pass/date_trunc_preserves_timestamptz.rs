use drizzle::core::expr::{date_trunc, raw_non_null, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

fn main() {
    let _: SQLExpr<'static, PostgresValue, drizzle::postgres::types::Timestamptz, NonNull, Scalar> =
        date_trunc("day", raw_non_null::<PostgresValue, drizzle::postgres::types::Timestamptz>("NOW()"));
}
