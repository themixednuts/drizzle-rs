use drizzle::core::expr::{abs, count_all, round, sum, window, NonNull, Null, SQLExpr, Scalar};
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // abs(sum(price)) preserves Agg — verify via .over() and type annotation
    let windowed: SQLExpr<'_, SQLiteValue, _, _, Scalar> =
        abs(sum(item.price)).over(window());
    let _ = windowed;

    // round(count_all()) preserves Agg
    let _: SQLExpr<'_, SQLiteValue, _, NonNull, Scalar> =
        round(count_all()).over(window());

    // Negation preserves Agg: -sum(price) is still Agg
    let _: SQLExpr<'_, SQLiteValue, _, Null, Scalar> =
        (-sum(item.price)).over(window());
}
