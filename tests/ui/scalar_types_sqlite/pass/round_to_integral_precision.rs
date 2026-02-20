use drizzle::core::expr::{raw_non_null, round_to};
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = round_to(
        raw_non_null::<SQLiteValue, drizzle::sqlite::types::Real>("1.234"),
        raw_non_null::<SQLiteValue, drizzle::sqlite::types::Integer>("2"),
    );
}
