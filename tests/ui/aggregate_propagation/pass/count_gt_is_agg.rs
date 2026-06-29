use drizzle::core::expr::{count, gt, window};
use drizzle::sqlite::prelude::*;

fn main() {
    // gt(count(()), 5) should preserve Agg, so .over() should be callable
    let _ = gt::<SQLiteValue, _, _>(count(()), 5i64).over(window());
}
