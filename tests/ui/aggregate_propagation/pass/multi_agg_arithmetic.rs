use drizzle::core::expr::{count, sum, window, Agg, SQLExpr};
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // sum(price) + count(()) — two aggregates combined remain Agg
    let combined: SQLExpr<'_, SQLiteValue, _, _, Agg> = sum(item.price) + count(());

    // The combined Agg expression can be windowed
    let _ = combined.over(window());
}
