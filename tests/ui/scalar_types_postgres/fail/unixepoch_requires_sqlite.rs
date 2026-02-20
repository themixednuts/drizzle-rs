use drizzle::core::expr::{raw_non_null, unixepoch};
use drizzle::postgres::prelude::*;

fn main() {
    let _ = unixepoch(raw_non_null::<PostgresValue, drizzle::postgres::types::Timestamp>("NOW()"));
}
