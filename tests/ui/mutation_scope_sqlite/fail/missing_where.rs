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
    let (db, Schema { user }) = Drizzle::new(conn);

    // A forgotten WHERE must not empty or rewrite the table.
    let _ = db.delete(user).execute();
    let _ = db
        .update(user)
        .set(UpdateUser::default().with_name("x"))
        .execute();
}
