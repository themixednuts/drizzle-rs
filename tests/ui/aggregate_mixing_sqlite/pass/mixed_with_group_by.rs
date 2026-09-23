use drizzle::core::expr::{alias, count};
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
    let (db, Schema { user, .. }) = Drizzle::new(conn);

    // Mixed scalar + aggregate WITH GROUP BY — should pass
    let _: drizzle::Result<Vec<MixedRow>> = db
        .select((user.name, alias(count(()), "total")))
        .from(user)
        .group_by(user.name)
        .all();
}
