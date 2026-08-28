use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    id: u64,
    #[column(VARCHAR(255))]
    name: String,
}

#[MySQLIndex]
struct UsersNameIdx(Users::name);

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users } = Schema::new();
    let _ = builder
        .select(users.id)
        .from(users)
        .use_index(UsersNameIdx::new())
        .force_index(UsersNameIdx::new());
}
