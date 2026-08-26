use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users { #[column(PRIMARY)] id: u64, name: String }
#[derive(MySQLSchema)]
struct Schema { users: Users }

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let _ = builder.update(users).set(UpdateUsers::default().with_name("Bob")).returning(users.id);
}
