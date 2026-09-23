use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
    name: String,
}

// A column without its table used to make the macro panic.
#[SQLiteView(
    query(
        select(id, User::name),
        from(User),
    ),
    NAME = "bad_view"
)]
struct BadView {
    id: i32,
    name: String,
}

fn main() {}
