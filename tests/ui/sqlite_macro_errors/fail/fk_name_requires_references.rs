use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i64,
    #[column(fk_name = "posts_author_fk")]
    author_id: i64,
}

fn main() {}
