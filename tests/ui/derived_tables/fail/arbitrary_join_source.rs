use drizzle::core::SQL;
use drizzle::sqlite::{helpers::JoinSource, prelude::SQLiteValue};

fn require_join_source<'a, Source: JoinSource<'a>>(_: Source) {}

fn main() {
    let raw: SQL<'_, SQLiteValue<'_>> = SQL::raw("bogus");
    require_join_source(raw);
}
