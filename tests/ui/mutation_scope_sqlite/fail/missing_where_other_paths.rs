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
    let (mut db, Schema { user }) = Drizzle::new(conn);

    // Preparing does not skip the WHERE either.
    let _ = db.delete(user).prepare();

    // Nor does running inside a transaction.
    let _ = db.transaction(Default::default(), |tx| {
        tx.update(user)
            .set(UpdateUser::default().with_name("x"))
            .execute()?;
        Ok(())
    });
}
