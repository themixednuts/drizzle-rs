use drizzle::core::expr::{count_all, is_not_null, is_null, nullif, sum, window, NonNull, SQLExpr, Scalar};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // is_null(sum(price)) preserves Agg
    let _: SQLExpr<'_, PostgresValue, drizzle::postgres::types::Boolean, NonNull, Scalar> =
        is_null(sum(item.price)).over(window());

    // is_not_null(count_all()) preserves Agg
    let _ = is_not_null(count_all::<PostgresValue>()).over(window());

    // nullif(sum(price), sum(price)) preserves Agg
    let _ = nullif(sum(item.price), sum(item.price)).over(window());
}
