use drizzle::postgres::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PostgresEnum)]
enum JobState {
    #[default]
    Queued,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PostgresEnum)]
#[repr(i64)]
enum NumericJobState {
    #[default]
    Queued = 0,
    Complete = 1,
}

#[PostgresTable]
struct Jobs {
    #[column(primary)]
    id: uuid::Uuid,
    #[column(enum)]
    state: JobState,
    #[column(enum)]
    numeric_state: NumericJobState,
}

fn assert_enum_row_traits<Row: ?Sized>()
where
    JobState: drizzle::core::FromDrizzleRow<Row> + drizzle::core::RowColumnList<Row>,
    Option<JobState>: drizzle::core::FromDrizzleRow<Row> + drizzle::core::RowColumnList<Row>,
    NumericJobState: drizzle::core::FromDrizzleRow<Row> + drizzle::core::RowColumnList<Row>,
    Option<NumericJobState>: drizzle::core::FromDrizzleRow<Row> + drizzle::core::RowColumnList<Row>,
{
}

fn main() {
    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    assert_enum_row_traits::<drizzle::postgres::Row>();

    #[cfg(feature = "aws-data-api")]
    assert_enum_row_traits::<drizzle::postgres::aws_data_api::Row>();
}
