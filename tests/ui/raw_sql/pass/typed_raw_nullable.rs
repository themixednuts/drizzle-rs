use drizzle::core::expr::{alias, raw_non_null, raw_nullable};
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
struct RawNullableRow {
    one: Option<i32>,
    two: Option<i32>,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    let _: drizzle::Result<Vec<RawNullableRow>> = db
        .select((
            alias(
                raw_non_null::<_, drizzle::sqlite::types::Integer>("1").nullable(),
                "one",
            ),
            alias(raw_nullable::<_, drizzle::sqlite::types::Integer>("NULL"), "two"),
        ))
        .from(user)
        .all();
}
