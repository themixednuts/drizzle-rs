use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    active: i32,
    name: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    // A condition tuple is an AND of its elements, so every element must be
    // boolean-like. Text is not — this must be rejected.
    let _ = db
        .select(())
        .from(user)
        .r#where((user.active, user.name))
        .to_sql();
}
