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

    let limit = user.id.placeholder("limit");
    let offset = user.id.placeholder("offset");

    let _ = db.select(()).from(user).limit(10).offset(0);
    let _ = db.select(()).from(user).limit(limit).offset(offset);
    let _ = db
        .select(())
        .from(user)
        .limit(drizzle::core::Placeholder::named("limit"));
}
