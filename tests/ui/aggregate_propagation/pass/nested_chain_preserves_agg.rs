use drizzle::core::expr::{abs, coalesce, gt, sum, window, NonNull, SQLExpr, Scalar};
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Item {
    #[column(primary)]
    id: i32,
    price: i32,
}

fn main() {
    let item = Item::default();

    // Deep chain: gt(abs(coalesce(sum(price), 0)), 100)
    // sum → Agg, coalesce(Agg, Scalar) → Agg, abs(Agg) → Agg, gt(Agg, Scalar) → Agg
    let deep_expr = gt(abs(coalesce(sum(item.price), 0)), 100);
    let _: SQLExpr<'_, SQLiteValue, _, NonNull, Scalar> = deep_expr.over(window());
}
