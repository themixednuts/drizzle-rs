use drizzle::core::expr::{sum, window};
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();
    // sum(price) + 5 should preserve Agg, so .over() should be callable
    let _ = (sum(item.price) + 5).over(window());
}
