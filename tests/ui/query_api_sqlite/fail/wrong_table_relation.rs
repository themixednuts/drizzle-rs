use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
    name: String,
}

#[SQLiteTable(NAME = "posts")]
struct Post {
    #[column(PRIMARY)]
    id: i32,
    title: String,
    #[column(REFERENCES = User::id)]
    author_id: i32,
}

fn main() {
    let post = Post::default();
    // Calling a Post relation on a User query should fail —
    // RelationDef<Source = User> is required but the reverse relation
    // generated from Post has Source = User for posts(), however
    // the forward relation post.author() has Source = Post.
    // Using post.author() on a User query should be rejected.
    let _ = drizzle::core::query::QueryBuilder::<drizzle::sqlite::values::SQLiteValue, User>::new()
        .with(post.author());
}
