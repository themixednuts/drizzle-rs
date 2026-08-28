use drizzle::core::expr::eq;
use drizzle::postgres::{builder::QueryBuilder, prelude::*};

#[PostgresTable]
struct Users {
    #[column(primary)]
    id: i32,
}

#[derive(PostgresSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let delete = builder
        .delete(users)
        .r#where(eq(users.id, 1))
        .returning(users.id);
    let _ = builder.insert(users).select(delete);
}
