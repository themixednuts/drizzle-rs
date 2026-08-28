extern crate drizzle_mysql as driver;

use drizzle::core::expr::eq;
use drizzle::mysql::prelude::*;
use driver::builder::QueryBuilder as DriverQueryBuilder;

mod select {}
#[allow(dead_code)]
struct QueryBuilder;

tag!(DerivedPosts, "derived_posts");

#[MySQLTable]
struct Users {
    #[column(PRIMARY)]
    id: i32,
}

#[MySQLTable]
struct Posts {
    #[column(PRIMARY)]
    id: i32,
    user_id: i32,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

fn main() {
    let Schema { users, posts } = Schema::new();
    let source = DriverQueryBuilder::new::<Schema>()
        .select(posts.user_id)
        .from(posts)
        .alias(DerivedPosts);
    let source_user_id = source.fields().0;

    let _ = DriverQueryBuilder::new::<Schema>()
        .select((users.id, source_user_id))
        .from(users)
        .left_join_lateral((source, eq(users.id, source_user_id)));
}
