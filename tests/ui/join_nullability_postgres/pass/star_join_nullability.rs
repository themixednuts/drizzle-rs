use drizzle::core::HasSelectModel;
use drizzle::postgres::builder::select::SelectBuilder;
use drizzle::postgres::builder::QueryBuilder;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
}

#[PostgresTable]
struct Posts {
    #[column(primary)]
    id: i32,
    #[column(references = Users::id)]
    user_id: i32,
    title: String,
}

#[derive(PostgresSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

type UserRow = <Users as HasSelectModel>::SelectModel;
type PostRow = <Posts as HasSelectModel>::SelectModel;

fn expect_inner<'a, St, T, M>(_: SelectBuilder<'a, Schema, St, T, M, (UserRow, PostRow)>) {}
fn expect_left<'a, St, T, M>(_: SelectBuilder<'a, Schema, St, T, M, (UserRow, Option<PostRow>)>) {}
fn expect_right<'a, St, T, M>(_: SelectBuilder<'a, Schema, St, T, M, (Option<UserRow>, PostRow)>) {}
fn expect_full<'a, St, T, M>(
    _: SelectBuilder<'a, Schema, St, T, M, (Option<UserRow>, Option<PostRow>)>,
) {
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();

    let Schema { users, posts } = Schema::new();
    expect_inner(qb.select(()).from(users).inner_join(posts));

    let Schema { users, posts } = Schema::new();
    expect_left(qb.select(()).from(users).left_join(posts));

    let Schema { users, posts } = Schema::new();
    expect_right(qb.select(()).from(users).right_join(posts));

    let Schema { users, posts } = Schema::new();
    expect_full(qb.select(()).from(users).full_join(posts));
}
