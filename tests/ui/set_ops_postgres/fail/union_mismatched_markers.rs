use drizzle::postgres::builder::QueryBuilder;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
}

#[derive(PostgresSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();
    let Schema { users: left_user } = Schema::new();
    let Schema { users: right_user } = Schema::new();

    // Marker/row mismatch: SELECT * vs SELECT column tuple.
    let _ = qb
        .select(())
        .from(left_user)
        .union(qb.select(right_user.id).from(right_user));
}
