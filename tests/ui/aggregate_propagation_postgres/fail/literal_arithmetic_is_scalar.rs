use drizzle::core::expr::window;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();
    // column + literal is Scalar + Scalar = Scalar — .over() should be rejected
    let _ = (item.price + 5).over(window());
}
