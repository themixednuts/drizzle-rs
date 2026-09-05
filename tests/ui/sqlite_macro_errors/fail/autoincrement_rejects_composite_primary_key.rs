use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Memberships {
    #[column(primary, autoincrement)]
    user_id: i64,
    #[column(primary)]
    group_id: i64,
}

fn main() {}
