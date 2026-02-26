use drizzle::core::expr::{sum, window, Scalar, SQLExpr};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // sum(price).over(window()) converts Agg → Scalar
    let windowed: SQLExpr<'_, PostgresValue, _, _, Scalar> = sum(item.price).over(window());

    // A Scalar windowed result can participate in arithmetic with other scalars
    let _ = windowed + item.price;
}
