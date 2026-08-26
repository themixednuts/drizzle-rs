use drizzle::core::{
    SQL,
    expr::{NonNull, SQLExpr, Scalar},
};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();
    let raw = String::from("1");
    let rhs: SQLExpr<'_, PostgresValue, drizzle::postgres::types::Int4, NonNull, Scalar> =
        SQLExpr::new(SQL::raw(raw.as_str()));

    // Generated column operators accept borrowed typed expressions without
    // forcing the expression lifetime to be 'static.
    let _ = item.price + &rhs;
}
