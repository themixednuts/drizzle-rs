use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users { #[column(PRIMARY)] id: u64 }
#[derive(MySQLSchema)]
struct Schema { users: Users }

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let _ = builder.delete(users).returning(users.id);
}
