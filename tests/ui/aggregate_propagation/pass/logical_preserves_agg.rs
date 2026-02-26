use drizzle::core::expr::{and2, count_all, gt, lt, window};
use drizzle::sqlite::prelude::*;

fn main() {
    // and2(gt(count_all(), 5), lt(count_all(), 100)) should preserve Agg
    let expr = and2(
        gt::<SQLiteValue, _, _>(count_all(), 5i64),
        lt::<SQLiteValue, _, _>(count_all(), 100i64),
    );
    let _ = expr.over(window());
}
