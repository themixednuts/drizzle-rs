use drizzle::core::expr::{date_trunc, raw_non_null};
use drizzle::core::types::Timestamp;
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = date_trunc(
        "day",
        raw_non_null::<SQLiteValue, Timestamp>("CURRENT_TIMESTAMP"),
    );
}
