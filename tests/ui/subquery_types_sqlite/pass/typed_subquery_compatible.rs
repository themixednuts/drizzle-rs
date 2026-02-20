use drizzle::core::expr::*;
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

    let Schema { users: outer_users } = Schema::new();
    let Schema {
        users: scalar_users,
    } = Schema::new();
    let ids = qb.select(scalar_users.id).from(scalar_users);
    let _ = qb
        .select(outer_users.id)
        .from(outer_users)
        .r#where(in_subquery(outer_users.id, ids));

    let Schema { users: left_users } = Schema::new();
    let Schema { users: right_users } = Schema::new();
    let right_rows = qb
        .select((right_users.id, right_users.name))
        .from(right_users);
    let _ = qb
        .select((left_users.id, left_users.name))
        .from(left_users)
        .r#where(in_subquery(
            row((left_users.id, left_users.name)),
            right_rows,
        ));
}
