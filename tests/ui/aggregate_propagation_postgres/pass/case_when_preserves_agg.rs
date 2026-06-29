use drizzle::core::expr::{case, count, gt, window, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

fn main() {
    // CASE WHEN with aggregate condition and literal result preserves Agg
    let expr = case::<PostgresValue>()
        .when(gt(count(()), 5i64), 1)
        .r#else(0);

    let _: SQLExpr<'_, PostgresValue, _, _, Scalar> = expr.over(window());
}
