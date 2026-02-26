use drizzle::core::expr::{between, sum, window, NonNull, SQLExpr, Scalar};
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // between(sum(price), 0, 100) — the aggregate operand makes the result Agg
    let _: SQLExpr<'_, SQLiteValue, drizzle::sqlite::types::Integer, NonNull, Scalar> =
        between(sum(item.price), 0, 100).over(window());
}
