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
    let qb = drizzle::postgres::builder::QueryBuilder::new::<Schema>();
    let Schema { user } = Schema::new();

    // Int4 is not BooleanLike in PostgreSQL — should fail
    let _ = qb.select(()).from(user).r#where(user.age);
}
