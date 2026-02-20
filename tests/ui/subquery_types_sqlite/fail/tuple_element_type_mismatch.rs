use drizzle::core::expr::{in_subquery, row};
use drizzle::sqlite::builder::QueryBuilder;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();

    let Schema { users: left_users } = Schema::new();
    let Schema { users: right_users } = Schema::new();

    let swapped_types = qb
        .select((right_users.name, right_users.id))
        .from(right_users);

    let _ = qb
        .select((left_users.id, left_users.name))
        .from(left_users)
        .r#where(in_subquery(
            row((left_users.id, left_users.name)),
            swapped_types,
        ));
}
