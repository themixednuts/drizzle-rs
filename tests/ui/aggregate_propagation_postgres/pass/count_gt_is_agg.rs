use drizzle::core::expr::{count_all, gt, window, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

fn main() {
    // gt(count_all(), 5) preserves Agg — comparison with aggregate operand
    let expr = gt::<PostgresValue, _, _>(count_all(), 5i64);

    let _: SQLExpr<'_, PostgresValue, _, NonNull, Scalar> = expr.over(window());
}
