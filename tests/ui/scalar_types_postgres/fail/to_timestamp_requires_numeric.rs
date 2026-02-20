use drizzle::core::expr::{raw_non_null, to_timestamp};
use drizzle::postgres::prelude::*;

fn main() {
    let _ = to_timestamp(raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'1700000000'"));
}
