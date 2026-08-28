use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    name: String,
}

#[MySQLTable]
struct Posts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    user_id: u64,
    title: String,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users, posts } = Schema::new();
    let selected = builder.select((users.id, users.name)).from(users);
    let _ = builder
        .insert(posts)
        .columns((users.id, posts.title))
        .select(selected);
}
