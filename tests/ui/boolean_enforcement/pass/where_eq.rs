use drizzle::core::expr::eq;
use drizzle::postgres::builder::QueryBuilder;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

#[derive(PostgresSchema)]
struct Schema {
    user: User,
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();
    let Schema { user } = Schema::new();

    // eq() returns Boolean — should be accepted by r#where
    let _ = qb.select(()).from(user).r#where(eq(user.id, 1));
}
