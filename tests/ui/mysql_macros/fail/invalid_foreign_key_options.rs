use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Parents {
    #[column(PRIMARY)]
    id: u64,
}

#[MySQLTable(TEMPORARY)]
struct TemporaryChildren {
    #[column(REFERENCES = Parents::id)]
    parent_id: u64,
}

#[MySQLTable]
struct NonNullableSetNull {
    #[column(ON_DELETE = SET_NULL, REFERENCES = Parents::id)]
    parent_id: u64,
}

fn main() {}
