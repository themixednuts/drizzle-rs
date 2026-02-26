use drizzle::core::expr::{raw_non_null, round_to};
use drizzle::postgres::prelude::*;

fn main() {
    // round_to() precision must be Integral, not Text
    let _ = round_to(
        raw_non_null::<PostgresValue, drizzle::postgres::types::Float8>("1.234"),
        raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'two'"),
    );
}
