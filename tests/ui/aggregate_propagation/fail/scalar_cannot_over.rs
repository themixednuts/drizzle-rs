use drizzle::core::expr::window;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();
    // A plain column is Scalar — .over() requires Agg and should be rejected
    let _ = item.price.over(window());
}
