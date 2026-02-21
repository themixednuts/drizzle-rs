use drizzle::core::expr::{and, eq, gt};
use drizzle::postgres::builder::QueryBuilder;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

#[derive(PostgresSchema)]
struct Schema {
    user: User,
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();
    let Schema { user } = Schema::new();

    // and([eq(...), gt(...)]) returns Boolean — should be accepted
    let _ = qb
        .select(())
        .from(user)
        .r#where(and([eq(user.id, 1), gt(user.age, 18)]));
}
