use drizzle::core::expr::{count_all, in_array, window, NonNull, SQLExpr, Scalar};
use drizzle::sqlite::prelude::*;

fn main() {
    // in_array(count_all(), [1, 2, 3]) — aggregate tested against scalar array
    let _: SQLExpr<'_, SQLiteValue, drizzle::sqlite::types::Integer, NonNull, Scalar> =
        in_array(count_all(), [1i64, 2, 3]).over(window());
}
