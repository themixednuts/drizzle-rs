use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
}

// An unknown action used to drop the ON DELETE clause without a word.
#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i32,
    #[column(references = Users::id, on_delete = EXPLODE)]
    user_id: i32,
}

fn main() {}
