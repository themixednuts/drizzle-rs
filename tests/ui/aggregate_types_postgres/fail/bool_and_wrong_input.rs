use drizzle::core::expr::{bool_and, raw_non_null};
use drizzle::postgres::prelude::*;

fn main() {
    let _ = bool_and(raw_non_null::<PostgresValue, drizzle::postgres::types::Int4>("1"));
}
