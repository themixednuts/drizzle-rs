use drizzle::core::expr::in_subquery;
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

    let right_ids = qb.select(right_users.id).from(right_users);

    let _ = qb
        .select((left_users.id, left_users.name))
        .from(left_users)
        .r#where(in_subquery(
            (left_users.id, left_users.name),
            right_ids,
        ));
}
