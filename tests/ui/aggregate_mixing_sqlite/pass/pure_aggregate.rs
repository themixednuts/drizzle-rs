use drizzle::core::expr::{alias, count_all, sum};
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
struct AggRow {
    total: i64,
    id_sum: Option<i64>,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    // Pure aggregate select — no GROUP BY needed
    let _: drizzle::Result<Vec<AggRow>> = db
        .select((alias(count_all(), "total"), alias(sum(user.id), "id_sum")))
        .from(user)
        .all();
}
