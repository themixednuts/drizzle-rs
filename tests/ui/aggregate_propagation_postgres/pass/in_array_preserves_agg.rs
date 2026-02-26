use drizzle::core::expr::{count_all, in_array, window, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

fn main() {
    // in_array(count_all(), [1, 2, 3]) — aggregate tested against scalar array
    let _: SQLExpr<'_, PostgresValue, _, NonNull, Scalar> =
        in_array(count_all(), [1i64, 2, 3]).over(window());
}
