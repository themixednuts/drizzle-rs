use drizzle::core::expr::{sum, window, Agg, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // sum(price) + 5 should preserve Agg
    let expr: SQLExpr<'_, PostgresValue, _, _, Agg> = sum(item.price) + 5;

    // Verify .over() converts Agg → Scalar
    let _: SQLExpr<'_, PostgresValue, _, _, Scalar> = expr.over(window());
}
