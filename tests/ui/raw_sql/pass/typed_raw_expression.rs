use drizzle::core::expr::{alias, raw_non_null};
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

#[derive(SQLiteFromRow)]
struct RawRow {
    one: i32,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    let _: drizzle::Result<Vec<RawRow>> = db
        .select(alias(
            raw_non_null::<_, drizzle::sqlite::types::Integer>("1"),
            "one",
        ))
        .from(user)
        .all();
}
