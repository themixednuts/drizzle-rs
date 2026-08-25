use drizzle::sqlite::prelude::*;

#[derive(SQLiteEnum, Clone, Debug, Default, PartialEq)]
#[repr(i64)]
enum JobState {
    #[default]
    Queued = 0,
    Complete = 1,
}

#[SQLiteTable]
struct Jobs {
    id: i64,
    #[column(text, ENUM)]
    state: JobState,
}

fn main() {}
