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
    let Schema { users: left_user } = Schema::new();
    let Schema { users: right_user } = Schema::new();

    let _ = qb
        .select(left_user.id)
        .from(left_user)
        .union(qb.select(right_user.id).from(right_user));
}
