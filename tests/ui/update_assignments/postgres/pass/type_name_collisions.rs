use drizzle::postgres::prelude::*;

struct PostgresUpdateValue;

#[PostgresTable]
struct Users {
    name: String,
}

fn main() {
    let _ = UpdateUsers::default().with_name("Ada");
}
