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

// Two links between users and posts mean different things, so both cannot
// be `users.posts()`: rustc reports the duplicate at each link's columns.
#[SQLiteTable(NAME = "post_likes")]
struct PostLike {
    #[column(REFERENCES = User::id)]
    user_id: i32,
    #[column(REFERENCES = Post::id)]
    post_id: i32,
}

#[SQLiteTable(NAME = "post_bookmarks")]
struct PostBookmark {
    #[column(REFERENCES = User::id)]
    user_id: i32,
    #[column(REFERENCES = Post::id)]
    post_id: i32,
}

fn main() {}
