use drizzle::core::expr::sqrt;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Readings {
    #[column(primary)]
    id: i64,
    value: f64,
}

fn main() {
    let readings = Readings::default();
    // Stock SQLite has no SQRT: without the `math` feature this must not compile.
    let _ = sqrt(readings.value);
}
