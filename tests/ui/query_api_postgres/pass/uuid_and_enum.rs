use drizzle::postgres::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PostgresEnum)]
enum JobState {
    #[default]
    Queued,
    Complete,
}

#[PostgresTable]
struct Jobs {
    #[column(primary)]
    id: uuid::Uuid,
    #[column(enum)]
    state: JobState,
}

fn main() {}
