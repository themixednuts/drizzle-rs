use drizzle::core::expr::{coalesce, sum, window};
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();
    // coalesce(sum(col), 0) should preserve Agg, so .over() should be callable
    let _ = coalesce(sum(item.price), 0).over(window());
}
