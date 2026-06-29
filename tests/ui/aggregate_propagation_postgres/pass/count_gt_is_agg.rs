use drizzle::core::expr::{count, gt, window, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

fn main() {
    // gt(count(()), 5) preserves Agg — comparison with aggregate operand
    let expr = gt::<PostgresValue, _, _>(count(()), 5i64);

    let _: SQLExpr<'_, PostgresValue, _, NonNull, Scalar> = expr.over(window());
}
