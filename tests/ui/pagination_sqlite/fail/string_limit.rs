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

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    let _ = db.select(()).from(user).limit("not a limit");
}
