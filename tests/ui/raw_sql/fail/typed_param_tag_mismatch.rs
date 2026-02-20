use drizzle::core::expr::eq;
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

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    let id = user.id.placeholder("id");

    let q = db.select(()).from(user).r#where(eq(user.id, id)).prepare();

    let _rows: Vec<SelectUser> = q
        .all(db.conn(), [id.bind("thisisntnumbersoshouldfail")])
        .unwrap();
}
