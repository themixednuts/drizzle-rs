use drizzle::core::expr::{alias, count_all};
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
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

    // Scalar column `name` not in GROUP BY (only `id` is grouped) — should fail
    let _: drizzle::Result<Vec<MixedRow>> = db
        .select((user.name, alias(count_all(), "total")))
        .from(user)
        .group_by(user.id)
        .all();
}
