use drizzle::core::expr::{between, eq, like};
use drizzle::core::Placeholder;
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

    let _: drizzle::Result<Vec<SelectUser>> = db
        .select(())
        .from(user)
        .r#where(eq(user.id, Placeholder::named("id")))
        .all();

    let _: drizzle::Result<Vec<SelectUser>> = db
        .select(())
        .from(user)
        .r#where(like(user.name, Placeholder::named("pattern")))
        .all();

    let _: drizzle::Result<Vec<SelectUser>> = db
        .select(())
        .from(user)
        .r#where(between(
            user.id,
            Placeholder::named("low"),
            Placeholder::named("high"),
        ))
        .all();
}
