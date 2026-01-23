//! Test that comparing Bool column with Int literal fails to compile.

use drizzle::sqlite::prelude::*;
use drizzle::core::expr::eq;

#[SQLiteTable]
struct Config {
    #[column(primary)]
    id: i32,
    enabled: bool,
}

fn main() {
    let config = Config::default();

    // ERROR: Bool is not Compatible<Int>
    let _ = eq(config.enabled, 42);
}
