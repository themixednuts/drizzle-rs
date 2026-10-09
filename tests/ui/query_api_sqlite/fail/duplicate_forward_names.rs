use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
}

#[SQLiteTable(NAME = "teams")]
struct Team {
    #[column(PRIMARY)]
    id: i32,
}

// `owner_id` and `owner` both give the forward accessor `owner()`.
#[SQLiteTable(NAME = "projects")]
struct Project {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = User::id)]
    owner_id: i32,
    #[column(REFERENCES = Team::id)]
    owner: i32,
}

fn main() {}
