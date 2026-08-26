use drizzle::core::expr::eq;
use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
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
    let delete = builder.delete(users).r#where(eq(users.id, 1_u64));
    let _ = builder.insert(users).select(delete);
}
