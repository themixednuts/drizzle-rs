use drizzle::core::expr::{count_all, gt, window};
use drizzle::sqlite::prelude::*;

fn main() {
    // gt(count_all(), 5) should preserve Agg, so .over() should be callable
    let _ = gt::<SQLiteValue, _, _>(count_all(), 5i64).over(window());
}
