use drizzle::core::expr::{raw_non_null, round_to};
use drizzle::core::types::{Double, Text};
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = round_to(
        raw_non_null::<SQLiteValue, Double>("1.234"),
        raw_non_null::<SQLiteValue, Text>("'two'"),
    );
}
