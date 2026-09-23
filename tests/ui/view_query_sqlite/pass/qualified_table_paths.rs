use drizzle::sqlite::prelude::*;

mod schema {
    use drizzle::sqlite::prelude::*;

    #[SQLiteTable(NAME = "users")]
    pub struct User {
        #[column(PRIMARY)]
        pub id: i32,
        pub name: String,
        // Any collation the application registers, named as an identifier.
        #[column(collate = my_collation)]
        pub handle: String,
    }
}

// Qualified table paths name the table by the segment before the column,
// in views and in indexes.
#[SQLiteView(
    query(
        select(schema::User::id, schema::User::name),
        from(schema::User),
        filter(eq(schema::User::name, "admin")),
    ),
    NAME = "admins"
)]
struct AdminsView {
    id: i32,
    name: String,
}

#[SQLiteIndex]
struct UserNameIdx(schema::User::name);

fn main() {
    let _sql = AdminsView::VIEW_DEFINITION_SQL;
    let _ddl = AdminsView::ddl_sql();
    let _index = UserNameIdx::new();
}
