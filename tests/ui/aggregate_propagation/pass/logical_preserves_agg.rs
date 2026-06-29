use drizzle::core::expr::{and, count, gt, lt, window};
use drizzle::sqlite::prelude::*;

fn main() {
    // and(gt(count(()), 5), lt(count(()), 100)) should preserve Agg
    let expr = and(
        gt::<SQLiteValue, _, _>(count(()), 5i64),
        lt::<SQLiteValue, _, _>(count(()), 100i64),
    );
    let _ = expr.over(window());
}
