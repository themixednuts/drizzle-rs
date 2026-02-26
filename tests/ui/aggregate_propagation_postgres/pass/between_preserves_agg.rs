use drizzle::core::expr::{between, sum, window, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // between(sum(price), 0, 100) — aggregate operand propagates Agg
    let _: SQLExpr<'_, PostgresValue, _, NonNull, Scalar> =
        between(sum(item.price), 0, 100).over(window());
}
