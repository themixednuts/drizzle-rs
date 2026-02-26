use drizzle::core::expr::{abs, count_all, round, sum, window, NonNull, Null, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // abs(sum(price)) preserves Agg
    let _: SQLExpr<'_, PostgresValue, _, _, Scalar> =
        abs(sum(item.price)).over(window());

    // round(count_all()) preserves Agg
    let _: SQLExpr<'_, PostgresValue, _, NonNull, Scalar> =
        round(count_all()).over(window());

    // Negation: -sum(price) preserves Agg
    let _: SQLExpr<'_, PostgresValue, _, Null, Scalar> =
        (-sum(item.price)).over(window());
}
