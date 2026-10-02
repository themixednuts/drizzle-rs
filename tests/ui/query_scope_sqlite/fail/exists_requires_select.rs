use drizzle::core::expr::exists;
use drizzle::sqlite::{builder::QueryBuilder, prelude::*};

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    // A column is not a subquery.
    let _ = qb.select(users.id).from(users).r#where(exists(users.id));
    // Neither is a DELETE, even with RETURNING.
    let _ = qb
        .select(users.id)
        .from(users)
        .r#where(exists(qb.delete(users).returning(users.id)));
}
