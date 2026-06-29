use drizzle::core::expr::{and, count, gt, lt, window, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

fn main() {
    // and(gt(count(()), 5), lt(count(()), 100)) preserves Agg
    let expr = and(
        gt::<PostgresValue, _, _>(count(()), 5i64),
        lt::<PostgresValue, _, _>(count(()), 100i64),
    );

    let _: SQLExpr<'_, PostgresValue, _, NonNull, Scalar> = expr.over(window());
}
