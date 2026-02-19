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

#[derive(SQLiteFromRow)]
#[from(User)]
struct UserRow {
    id: i32,
    name: String,
}

struct UTag;
impl drizzle::core::Tag for UTag {
    const NAME: &'static str = "u";
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, _schema) = Drizzle::new(conn, Schema::default());

    let u = User::alias::<UTag>();
    let _rows: Vec<UserRow> = db.select(UserRow::Select).from(u).all().unwrap();
}
