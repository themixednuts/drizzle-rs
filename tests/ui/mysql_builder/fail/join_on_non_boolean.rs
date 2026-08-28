use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
}

#[MySQLTable]
struct Posts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    user_id: u64,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users, posts } = Schema::new();
    let _ = builder
        .select(users.id)
        .from(users)
        .inner_join((posts, posts.user_id));
}
