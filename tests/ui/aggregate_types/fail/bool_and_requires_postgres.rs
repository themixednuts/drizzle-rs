use drizzle::core::expr::{bool_and, raw_non_null};
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = bool_and(raw_non_null::<SQLiteValue, drizzle::sqlite::types::Integer>("1"));
}
