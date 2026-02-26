use drizzle::core::expr::{length, max, upper, window, Null, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let item = Item::default();

    // upper(max(name)) preserves Agg
    let _: SQLExpr<'_, PostgresValue, drizzle::postgres::types::Text, Null, Scalar> =
        upper(max(item.name)).over(window());

    // length(max(name)) preserves Agg
    let _ = length(max(item.name)).over(window());
}
