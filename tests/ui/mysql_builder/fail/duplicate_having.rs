use drizzle::core::expr::{count, gt};
use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users { #[column(PRIMARY)] id: u64 }
#[derive(MySQLSchema)]
struct Schema { users: Users }

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let _ = builder.select(users.id).from(users).group_by(users.id)
        .having(gt(count(users.id), 0)).having(gt(count(users.id), 1));
}
