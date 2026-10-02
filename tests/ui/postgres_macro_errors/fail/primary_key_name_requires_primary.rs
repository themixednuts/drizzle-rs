use drizzle::postgres::prelude::*;

// `primary_key(name = ...)` names the key the `primary` fields form.
#[PostgresTable(primary_key(name = "posts_pk"))]
struct Posts {
    id: i32,
    title: String,
}

fn main() {}
