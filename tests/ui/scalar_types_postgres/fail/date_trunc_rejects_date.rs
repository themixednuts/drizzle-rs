use drizzle::core::expr::{date_trunc, raw_non_null};
use drizzle::postgres::prelude::*;

fn main() {
    let _ = date_trunc("day", raw_non_null::<PostgresValue, drizzle::postgres::types::Date>("CURRENT_DATE"));
}
