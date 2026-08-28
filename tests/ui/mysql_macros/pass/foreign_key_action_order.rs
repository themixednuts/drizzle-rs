use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Parents {
    #[column(PRIMARY)]
    id: u64,
}

#[MySQLTable]
struct Children {
    #[column(ON_DELETE = CASCADE, ON_UPDATE = RESTRICT, REFERENCES = Parents::id)]
    parent_id: u64,
}

fn main() {
    let sql = Children::create_table_sql();
    assert!(sql.contains("ON DELETE CASCADE ON UPDATE RESTRICT"));
}
