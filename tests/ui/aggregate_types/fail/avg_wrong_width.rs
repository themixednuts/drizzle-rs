use drizzle::core::expr::{alias, avg};
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

#[derive(SQLiteFromRow)]
struct AvgRow {
    value: Option<f32>,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    let _rows: Vec<AvgRow> = db
        .select(alias(avg(user.age), "value"))
        .from(user)
        .all()
        .unwrap();
}
