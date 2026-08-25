use drizzle::sqlite::prelude::*;

#[derive(SQLiteEnum, Clone, Debug, Default, PartialEq)]
enum JobState {
    #[default]
    Queued,
    Complete,
}

#[SQLiteTable]
struct Jobs {
    id: i64,
    #[column(integer, ENUM)]
    state: JobState,
}

fn main() {}
