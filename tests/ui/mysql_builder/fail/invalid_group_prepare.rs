use drizzle::core::expr::count;
use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    name: String,
    active: bool,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let _ = builder
        .select((users.name, count(users.id)))
        .from(users)
        .group_by(users.active)
        .prepare();
}
