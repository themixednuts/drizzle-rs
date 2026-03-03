use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    active: bool,
}

#[SQLiteView(
    query(
        select(User::id, User::name),
        from(User),
        filter(eq(User::active, true)),
    ),
    NAME = "active_users"
)]
struct ActiveUsersView {
    id: i32,
    name: String,
}

fn main() {
    let _sql = ActiveUsersView::VIEW_DEFINITION_SQL;
    let _ddl = ActiveUsersView::ddl_sql();
}
