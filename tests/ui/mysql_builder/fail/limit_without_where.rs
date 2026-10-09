use drizzle::mysql::builder::QueryBuilder;
use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Users {
    #[column(PRIMARY)]
    id: u64,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    // ORDER BY and LIMIT start from the WHERE: `.r#where(true).limit(1)`.
    let _ = builder.delete(users).limit(1);
}
