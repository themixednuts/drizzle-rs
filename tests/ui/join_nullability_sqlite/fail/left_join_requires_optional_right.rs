use drizzle::core::HasSelectModel;
use drizzle::sqlite::builder::select::SelectBuilder;
use drizzle::sqlite::builder::QueryBuilder;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
}

#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i32,
    #[column(references = Users::id)]
    user_id: i32,
    title: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

type UserRow = <Users as HasSelectModel>::SelectModel;
type PostRow = <Posts as HasSelectModel>::SelectModel;

fn expect_left_wrong<'a, St, T, M>(_: SelectBuilder<'a, Schema, St, T, M, (UserRow, PostRow)>) {}

fn main() {
    let qb = QueryBuilder::new::<Schema>();
    let Schema { users, posts } = Schema::new();

    // LEFT JOIN must make the right row optional.
    expect_left_wrong(qb.select(()).from(users).left_join(posts));
}
