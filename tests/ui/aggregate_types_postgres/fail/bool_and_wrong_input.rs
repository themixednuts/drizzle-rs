use drizzle::core::expr::{bool_and, raw_non_null};
use drizzle::core::types::Int;
use drizzle::postgres::prelude::*;

fn main() {
    let _ = bool_and(raw_non_null::<PostgresValue, Int>("1"));
}
