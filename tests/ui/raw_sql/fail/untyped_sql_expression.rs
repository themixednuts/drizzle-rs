use drizzle::core::expr::alias;
use drizzle::sql;
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

    let _rows: Vec<RawRow> = db.select(alias(sql!("1"), "one")).from(user).all().unwrap();
}
