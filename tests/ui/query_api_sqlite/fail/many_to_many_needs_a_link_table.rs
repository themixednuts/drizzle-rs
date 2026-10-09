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

#[SQLiteTable(NAME = "boards")]
struct Board {
    #[column(PRIMARY)]
    id: i32,
}

// Three foreign keys: there is no single other side for `many_to_many`.
#[SQLiteTable(NAME = "pins")]
struct Pin {
    #[column(REFERENCES = User::id, MANY_TO_MANY = "pinned_posts")]
    user_id: i32,
    #[column(REFERENCES = Post::id)]
    post_id: i32,
    #[column(REFERENCES = Board::id)]
    board_id: i32,
}

fn main() {}
