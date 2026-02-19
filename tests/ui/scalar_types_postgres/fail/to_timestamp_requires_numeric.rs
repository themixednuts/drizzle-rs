use drizzle::core::expr::{raw_non_null, to_timestamp};
use drizzle::core::types::Text;
use drizzle::postgres::prelude::*;

fn main() {
    let _ = to_timestamp(raw_non_null::<PostgresValue, Text>("'1700000000'"));
}
