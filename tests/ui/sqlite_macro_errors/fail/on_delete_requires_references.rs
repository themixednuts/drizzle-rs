use drizzle::sqlite::prelude::*;

// The error points at `on_delete`, not at the first option of the attribute.
#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i32,
    #[column(unique, on_delete = CASCADE)]
    user_id: i32,
}

fn main() {}
