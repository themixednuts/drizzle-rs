use drizzle::core::expr::{case, count, gt, window};
use drizzle::sqlite::prelude::*;

fn main() {
    // CASE WHEN with aggregate condition/result should preserve Agg
    let expr = case::<SQLiteValue>()
        .when(gt(count(()), 5i64), 1)
        .r#else(0);
    let _ = expr.over(window());
}
