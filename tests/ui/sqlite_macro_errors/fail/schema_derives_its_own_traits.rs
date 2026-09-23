use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
}

// `SQLiteSchema` implements Debug itself; deriving it again is E0119.
#[derive(SQLiteSchema)]
#[derive(Debug)]
struct Schema {
    users: Users,
}

fn main() {}
