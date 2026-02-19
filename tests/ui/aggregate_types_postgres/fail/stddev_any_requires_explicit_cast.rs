use drizzle::core::expr::{raw_non_null, stddev_pop};
use drizzle::core::types::Any;
use drizzle::postgres::prelude::*;

fn main() {
    let _ = stddev_pop(raw_non_null::<PostgresValue, Any>("'1'"));
}
