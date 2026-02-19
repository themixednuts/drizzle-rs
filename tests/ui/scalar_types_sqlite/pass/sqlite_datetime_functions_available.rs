use drizzle::core::expr::{date, datetime, julianday, raw_non_null, strftime, time, unixepoch};
use drizzle::core::types::Timestamp;
use drizzle::sqlite::prelude::*;

fn main() {
    let _ = date(raw_non_null::<SQLiteValue, Timestamp>("CURRENT_TIMESTAMP"));
    let _ = time(raw_non_null::<SQLiteValue, Timestamp>("CURRENT_TIMESTAMP"));
    let _ = datetime(raw_non_null::<SQLiteValue, Timestamp>("CURRENT_TIMESTAMP"));
    let _ = strftime(
        "%Y",
        raw_non_null::<SQLiteValue, Timestamp>("CURRENT_TIMESTAMP"),
    );
    let _ = julianday(raw_non_null::<SQLiteValue, Timestamp>("CURRENT_TIMESTAMP"));
    let _ = unixepoch(raw_non_null::<SQLiteValue, Timestamp>("CURRENT_TIMESTAMP"));
}
