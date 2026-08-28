use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    id: u64,
    #[column(VARCHAR(255))]
    name: String,
}

#[MySQLTable]
struct Posts {
    id: u64,
    user_id: u64,
}

#[MySQLIndex]
struct UsersNameIdx(Users::name);

#[MySQLIndex]
struct PostsUserIdIdx(Posts::user_id);

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users, .. } = Schema::new();
    let _ = builder
        .select(users.id)
        .from(users)
        .use_index((UsersNameIdx::new(), PostsUserIdIdx::new()));
}
