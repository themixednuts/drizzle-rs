use drizzle::core::expr::{raw_non_null, to_timestamp};
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = to_timestamp(raw_non_null::<SQLiteValue, drizzle::sqlite::types::Integer>("1700000000"));
}
