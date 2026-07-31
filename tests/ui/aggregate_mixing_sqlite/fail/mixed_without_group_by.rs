use drizzle::core::expr::{alias, count};
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
    email: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

#[derive(SQLiteFromRow)]
struct MixedRow {
    name: String,
    total: i64,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    // Scalar column `name` not in GROUP BY (only non-key `email` is grouped) —
    // should fail. Grouping by a non-primary-key column does not functionally
    // determine the table's other columns.
    let _: drizzle::Result<Vec<MixedRow>> = db
        .select((user.name, alias(count(()), "total")))
        .from(user)
        .group_by(user.email)
        .all();
}
