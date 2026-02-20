use drizzle::core::expr::{array_agg, raw_non_null};
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = array_agg(raw_non_null::<SQLiteValue, drizzle::sqlite::types::Integer>("1"));
}
