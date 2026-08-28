use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users { #[column(PRIMARY, AUTO_INCREMENT)] id: u64, name: String }
#[derive(MySQLSchema)]
struct Schema { users: Users }

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let _ = builder.insert(users).value(InsertUsers::new("Alice")).returning(users.id);
}
