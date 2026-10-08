use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
}

#[SQLiteTable(NAME = "posts")]
struct Post {
    #[column(PRIMARY)]
    id: i32,
}

// The link's rows are `users.post_likes()`, so the many-to-many accessor
// cannot take that name too.
#[SQLiteTable(NAME = "post_likes")]
struct PostLike {
    #[column(REFERENCES = User::id, MANY_TO_MANY = "post_likes")]
    user_id: i32,
    #[column(REFERENCES = Post::id)]
    post_id: i32,
}

fn main() {}
