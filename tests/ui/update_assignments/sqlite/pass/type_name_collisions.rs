use drizzle::sqlite::prelude::*;

struct SQLiteUpdateValue;

#[SQLiteTable]
struct Users {
    name: String,
}

fn main() {
    let _ = UpdateUsers::default().with_name("Ada");
}
