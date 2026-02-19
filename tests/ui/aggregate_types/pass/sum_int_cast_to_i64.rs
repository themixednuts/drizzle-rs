use drizzle::core::expr::{alias, cast, sum};
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

#[derive(SQLiteFromRow)]
struct SumRow {
    total: Option<i64>,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::default());

    let _: drizzle::Result<Vec<SumRow>> = db
        .select(alias(
            cast::<_, _, drizzle::core::types::BigInt>(
                sum(user.age),
                drizzle::sqlite::types::Integer,
            ),
            "total",
        ))
        .from(user)
        .all();
}
