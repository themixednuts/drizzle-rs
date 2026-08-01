use drizzle::core::expr::{all, any, eq, gt};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    active: bool,
    age: i32,
    name: String,
}

#[derive(PostgresSchema)]
struct Schema {
    user: User,
}

fn main() {
    let qb = drizzle::postgres::builder::QueryBuilder::new::<Schema>();
    let Schema { user } = Schema::new();

    // A tuple of boolean expressions is a condition list.
    let _ = qb
        .select(())
        .from(user)
        .r#where((user.active, gt(user.age, 18)));

    // Tuples nest inside or(), and all/any take the same lists.
    let _ = qb.select(()).from(user).r#where(any((
        (user.active, gt(user.age, 18)),
        all((eq(user.name, "root"), user.active)),
    )));

    // Optional elements are allowed.
    let _ = qb
        .select(())
        .from(user)
        .r#where((gt(user.age, 1), Some(eq(user.name, "root"))));
}
