use drizzle::core::expr::{bool_and, raw_non_null};
use drizzle::core::types::Bool;
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = bool_and(raw_non_null::<SQLiteValue, Bool>("1"));
}
