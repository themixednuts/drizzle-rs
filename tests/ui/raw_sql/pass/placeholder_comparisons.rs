use drizzle::core::expr::{between, eq, like};
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

    let id = user.id.placeholder("id");
    let pattern = user.name.placeholder("pattern");
    let low = user.id.placeholder("low");
    let high = user.id.placeholder("high");

    let by_id = db.select(()).from(user).r#where(eq(user.id, id)).prepare();
    let _: drizzle::Result<Vec<SelectUser>> = by_id.all(db.conn(), [id.bind(1)]);

    let by_pattern = db
        .select(())
        .from(user)
        .r#where(like(user.name, pattern))
        .prepare();
    let _: drizzle::Result<Vec<SelectUser>> = by_pattern.all(db.conn(), [pattern.bind("%a%")]);

    let by_range = db
        .select(())
        .from(user)
        .r#where(between(user.id, low, high))
        .prepare();
    let _: drizzle::Result<Vec<SelectUser>> = by_range.all(db.conn(), [low.bind(1), high.bind(10)]);
}
