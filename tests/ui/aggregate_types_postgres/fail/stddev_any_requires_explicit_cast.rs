use drizzle::core::expr::{raw_non_null, stddev_pop};
use drizzle::postgres::prelude::*;

fn main() {
    let _ = stddev_pop(raw_non_null::<PostgresValue, drizzle::postgres::types::Any>("'1'"));
}
