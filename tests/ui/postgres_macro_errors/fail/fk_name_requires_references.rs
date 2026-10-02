use drizzle::postgres::prelude::*;

// `fk_name` names the column's foreign key, so it needs one.
#[PostgresTable]
struct Posts {
    #[column(primary)]
    id: i32,
    #[column(fk_name = "posts_user_fk")]
    user_id: i32,
}

fn main() {}
