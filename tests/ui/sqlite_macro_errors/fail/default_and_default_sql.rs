use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct BadDefaults {
    #[column(default = 1, default_sql = "CURRENT_TIMESTAMP")]
    value: i32,
}

fn main() {}
