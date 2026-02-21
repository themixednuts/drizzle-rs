use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

#[derive(PostgresSchema)]
struct Schema {
    user: User,
}

fn main() {
    let user = User::default();
    // Text column is not BooleanLike — should fail
    let _ = drizzle::postgres::helpers::select(())
        .from(user)
        .r#where(user.name);
}
