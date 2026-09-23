use drizzle::sqlite::prelude::*;

// A reference that is not `Table::column` used to drop the foreign key.
#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i32,
    #[column(references = user_id)]
    user_id: i32,
}

fn main() {}
