use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
    manager_id: Option<i32>,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
}

struct Manager;
impl drizzle::core::Tag for Manager {
    const NAME: &'static str = "manager";
}

struct Peer;
impl drizzle::core::Tag for Peer {
    const NAME: &'static str = "peer";
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { users }) = Drizzle::new(conn);
    let manager = Users::alias::<Manager>();
    let peer = Users::alias::<Peer>();

    // `peer` is a different alias of the same table; it was never joined.
    let _rows: drizzle::Result<Vec<(String, String)>> = db
        .select((manager.name, peer.name))
        .from(users)
        .join((manager, eq(users.manager_id, manager.id)))
        .all();
}
