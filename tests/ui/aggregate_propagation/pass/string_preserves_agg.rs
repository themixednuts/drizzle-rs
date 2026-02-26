use drizzle::core::expr::{length, max, upper, window, Null, SQLExpr, Scalar};
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let item = Item::default();

    // upper(max(name)) preserves Agg — max returns Agg, upper wraps it
    let _: SQLExpr<'_, SQLiteValue, drizzle::sqlite::types::Text, Null, Scalar> =
        upper(max(item.name)).over(window());

    // length(max(name)) preserves Agg
    let _ = length(max(item.name)).over(window());
}
