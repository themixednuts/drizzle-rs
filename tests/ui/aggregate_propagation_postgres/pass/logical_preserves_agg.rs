use drizzle::core::expr::{and, count_all, gt, lt, window, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

fn main() {
    // and(gt(count_all(), 5), lt(count_all(), 100)) preserves Agg
    let expr = and(
        gt::<PostgresValue, _, _>(count_all(), 5i64),
        lt::<PostgresValue, _, _>(count_all(), 100i64),
    );

    let _: SQLExpr<'_, PostgresValue, _, NonNull, Scalar> = expr.over(window());
}
