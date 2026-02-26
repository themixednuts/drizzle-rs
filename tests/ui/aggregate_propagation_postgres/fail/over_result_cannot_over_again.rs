use drizzle::core::expr::{sum, window};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();
    // sum(price).over(window()) returns Scalar — calling .over() again should fail
    let _ = sum(item.price).over(window()).over(window());
}
