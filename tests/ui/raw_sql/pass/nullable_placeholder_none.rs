use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    nickname: Option<String>,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    let nickname = user.nickname.placeholder("nickname");
    let q = db
        .select(())
        .from(user)
        .r#where(eq(user.nickname, nickname))
        .prepare();

    let _: drizzle::Result<Vec<SelectUser>> = q.all(db.conn(), [nickname.bind_opt(None::<String>)]);
}
