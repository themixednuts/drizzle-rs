use drizzle::core::expr::{array_agg, raw_non_null};
use drizzle::core::types::Int;
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = array_agg(raw_non_null::<SQLiteValue, Int>("1"));
}
