use drizzle::core::expr::{coalesce, sum, window, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // coalesce(sum(price), 0) preserves Agg
    let _: SQLExpr<'_, PostgresValue, _, _, Scalar> =
        coalesce(sum(item.price), 0).over(window());
}
