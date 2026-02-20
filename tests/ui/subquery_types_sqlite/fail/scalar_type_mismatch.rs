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

    let Schema { users: outer_users } = Schema::new();
    let Schema { users: inner_users } = Schema::new();

    let names = qb.select(inner_users.name).from(inner_users);

    let _ = qb
        .select(outer_users.id)
        .from(outer_users)
        .r#where(in_subquery(outer_users.id, names));
}
