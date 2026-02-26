use drizzle::core::expr::{raw_non_null, substr};
use drizzle::postgres::prelude::*;

fn main() {
    // SUBSTR position and length must be Integral, not Text
    let _ = substr(
        raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'hello'"),
        raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'one'"),
        raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'two'"),
    );
}
