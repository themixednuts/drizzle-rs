use drizzle::core::expr::{raw_non_null, stddev_pop};
use drizzle::core::types::Int;
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = stddev_pop(raw_non_null::<SQLiteValue, Int>("1"));
}
