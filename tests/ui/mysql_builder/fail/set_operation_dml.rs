use drizzle::core::expr::eq;
use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let delete = builder.delete(users).r#where(eq(users.id, 1_u64));
    let other = builder.select(users.id).from(users);
    let _ = delete.union(other);
}
