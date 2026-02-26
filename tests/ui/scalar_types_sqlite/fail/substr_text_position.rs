use drizzle::core::expr::{raw_non_null, substr};
use drizzle::sqlite::prelude::*;

fn main() {
    // SUBSTR position and length must be Integral, not Text
    let _ = substr(
        raw_non_null::<SQLiteValue, drizzle::sqlite::types::Text>("'hello'"),
        raw_non_null::<SQLiteValue, drizzle::sqlite::types::Text>("'one'"),
        raw_non_null::<SQLiteValue, drizzle::sqlite::types::Text>("'two'"),
    );
}
