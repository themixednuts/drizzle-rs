use drizzle::core::expr::{all, any, eq, gt};
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

    // SQLite Integer is BooleanLike, so a tuple of integer conditions is a
    // condition list.
    let _ = db
        .select(())
        .from(user)
        .r#where((user.active, gt(user.id, 10)))
        .to_sql();

    // Tuples nest inside or(), and all/any take the same lists.
    let _ = db
        .select(())
        .from(user)
        .r#where(any((
            (user.active, gt(user.id, 10)),
            all((eq(user.name, "root"), user.active)),
        )))
        .to_sql();

    // Optional elements are allowed.
    let _ = db
        .select(())
        .from(user)
        .r#where((gt(user.id, 1), Some(eq(user.name, "root"))))
        .to_sql();
}
