use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    id: u64,
}

#[MySQLTable]
struct Posts {
    id: u64,
}

#[MySQLIndex]
struct PostsIdIdx(Posts::id);

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
        .use_index(PostsIdIdx::new());
}
