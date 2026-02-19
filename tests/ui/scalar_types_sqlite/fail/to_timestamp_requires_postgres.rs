use drizzle::core::expr::{raw_non_null, to_timestamp};
use drizzle::core::types::BigInt;
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = to_timestamp(raw_non_null::<SQLiteValue, BigInt>("1700000000"));
}
