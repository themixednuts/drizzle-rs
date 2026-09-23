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
    let Schema { users } = Schema::new();

    // SQLite has no INTERSECT ALL / EXCEPT ALL (only UNION ALL), so the
    // builder must not offer them.
    let _ = qb
        .select(users.name)
        .from(users)
        .intersect_all(qb.select(users.name).from(users));
    let _ = qb
        .select(users.name)
        .from(users)
        .except_all(qb.select(users.name).from(users));
}
