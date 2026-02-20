use drizzle::core::expr::{raw_non_null, stddev_pop};
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = stddev_pop(raw_non_null::<SQLiteValue, drizzle::sqlite::types::Integer>("1"));
}
