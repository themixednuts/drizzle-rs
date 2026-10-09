use drizzle::sqlite::builder::QueryBuilder;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(PRIMARY)]
    id: i32,
}

#[SQLiteTable]
struct Posts {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = Users::id)]
    author_id: i32,
    #[column(REFERENCES = Users::id)]
    editor_id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();
    let Schema { users, posts } = Schema::new();
    // Author or editor? Neither direction picks a key.
    let _ = qb.select(users.id).from(users).join(posts);
    let _ = qb.select(posts.id).from(posts).join(users);
}
