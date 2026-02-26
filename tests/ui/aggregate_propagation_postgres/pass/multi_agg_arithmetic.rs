use drizzle::core::expr::{count_all, sum, window, Agg, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // sum(price) + count_all() — two aggregates combined remain Agg
    let combined: SQLExpr<'_, PostgresValue, _, _, Agg> = sum(item.price) + count_all();

    // The combined Agg expression can be windowed
    let _: SQLExpr<'_, PostgresValue, _, _, Scalar> = combined.over(window());
}
