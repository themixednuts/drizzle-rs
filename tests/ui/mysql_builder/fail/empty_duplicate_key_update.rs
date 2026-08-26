use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    id: u64,
    name: String,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let _ = builder
        .insert(users)
        .value(InsertUsers::new(1, "Alice"))
        .on_duplicate_key_update(UpdateUsers::default());
}
