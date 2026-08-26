use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Users {
    id: u64,
}

#[MySQLIndex(algorithm = "concurrently")]
struct UsersIdIdx(Users::id);

fn main() {}
