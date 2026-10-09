use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i64,
}

// Deleting a user would have to set `user_id` to NULL, which it cannot hold.
#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i64,
    #[column(references = Users::id, on_delete = SET_NULL)]
    user_id: i64,
}

fn main() {}
