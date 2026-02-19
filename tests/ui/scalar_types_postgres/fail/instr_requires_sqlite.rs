use drizzle::core::expr::{instr, raw_non_null};
use drizzle::core::types::Text;
use drizzle::postgres::prelude::*;

fn main() {
    let _ = instr(
        raw_non_null::<PostgresValue, Text>("'hello'"),
        raw_non_null::<PostgresValue, Text>("'ll'"),
    );
}
