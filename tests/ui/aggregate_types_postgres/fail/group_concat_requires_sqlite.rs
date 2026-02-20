use drizzle::core::expr::{group_concat, raw_non_null};
use drizzle::postgres::prelude::*;

fn main() {
    let _ = group_concat(raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'x'"));
}
