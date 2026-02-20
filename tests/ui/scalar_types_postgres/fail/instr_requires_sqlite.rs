use drizzle::core::expr::{instr, raw_non_null};
use drizzle::postgres::prelude::*;

fn main() {
    let _ = instr(
        raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'hello'"),
        raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'ll'"),
    );
}
