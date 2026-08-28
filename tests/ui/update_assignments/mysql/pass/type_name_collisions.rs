use drizzle::mysql::prelude::*;

struct MySQLUpdateValue;

#[MySQLTable]
struct Users {
    name: String,
}

fn main() {
    let _ = UpdateUsers::default().with_name("Ada");
}
