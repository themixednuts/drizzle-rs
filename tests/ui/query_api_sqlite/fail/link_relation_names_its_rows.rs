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

// On a link table `relation` names the many-to-many accessor, and the link's
// rows keep `users.post_likes()`, so this names both the same.
#[SQLiteTable(NAME = "post_likes")]
struct PostLike {
    #[column(REFERENCES = User::id, RELATION = "post_likes")]
    user_id: i32,
    #[column(REFERENCES = Post::id)]
    post_id: i32,
}

fn main() {}
