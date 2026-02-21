use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

#[derive(PostgresSchema)]
struct Schema {
    user: User,
}

fn main() {
    let user = User::default();
    // Int4 is not BooleanLike in PostgreSQL — should fail
    let _ = drizzle::postgres::helpers::select(())
        .from(user)
        .r#where(user.age);
}
