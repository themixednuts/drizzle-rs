use drizzle::core::expr::{all, gt, lt, window};
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();
    // Every element is Scalar, so the list is Scalar too — .over() requires Agg
    // and must reject it. The fold does not silently widen to Agg.
    let conjunction = all::<SQLiteValue, _>((gt(item.price, 5), lt(item.price, 100)));
    let _ = conjunction.over(window());
}
