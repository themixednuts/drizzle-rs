use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    active: bool,
    age: i32,
}

#[derive(PostgresSchema)]
struct Schema {
    user: User,
}

fn main() {
    let qb = drizzle::postgres::builder::QueryBuilder::new::<Schema>();
    let Schema { user } = Schema::new();

    // A condition tuple is an AND of its elements. Int4 is not BooleanLike in
    // PostgreSQL, so this must be rejected.
    let _ = qb.select(()).from(user).r#where((user.active, user.age));
}
