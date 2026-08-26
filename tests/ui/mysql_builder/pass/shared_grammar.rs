use drizzle::core::expr::{alias, avg, char_length, eq, sum};
use drizzle::core::HasSelectModel;
use drizzle::mysql::builder::select::SelectBuilder;
use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(255))]
    name: String,
}

#[MySQLTable]
struct Posts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(REFERENCES = Users::id)]
    user_id: u64,
    title: String,
}

#[MySQLIndex]
struct UsersNameIdx(Users::name);

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

type UserRow = <Users as HasSelectModel>::SelectModel;
type PostRow = <Posts as HasSelectModel>::SelectModel;

fn expect_inner<'a, State, Table, Marker>(
    _: SelectBuilder<'a, Schema, State, Table, Marker, (UserRow, PostRow)>,
) {
}

fn expect_left<'a, State, Table, Marker>(
    _: SelectBuilder<'a, Schema, State, Table, Marker, (UserRow, Option<PostRow>)>,
) {
}

fn expect_right<'a, State, Table, Marker>(
    _: SelectBuilder<'a, Schema, State, Table, Marker, (Option<UserRow>, PostRow)>,
) {
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { users, .. } = Schema::new();

    let _ = builder.select(()).from(users);
    let _ = builder.select(users.id + 1_u64).from(users);
    let _ = builder.select(users.id / users.id).from(users);
    let aggregate = sum(users.id);
    let _ = builder.select(users.id + &aggregate).from(users);
    let _ = builder
        .select((sum(users.id), avg(users.id), char_length(users.name)))
        .from(users);
    let _ = builder
        .select_distinct(users.name)
        .from(users)
        .r#where(eq(users.id, 1_u64))
        .order_by(asc(users.id))
        .limit(10)
        .offset(2)
        .prepare();
    let _ = builder.insert(users).value(InsertUsers::new("Alice"));
    let _ = builder
        .insert(users)
        .ignore()
        .value(InsertUsers::new("Alice"));
    let _ = builder
        .insert(users)
        .value(InsertUsers::new("Alice"))
        .on_duplicate_key_update(UpdateUsers::default().with_name("updated"));
    let selected = builder.select((users.id, users.name)).from(users);
    let _ = builder.insert(users).select(selected);
    let _ = builder
        .update(users)
        .set(UpdateUsers::default().with_name("Bob"))
        .order_by(asc(users.id))
        .limit(1);
    let _ = builder.delete(users).order_by(asc(users.id)).limit(1);
    let _ = builder
        .select(users.id)
        .from(users)
        .use_index(UsersNameIdx::new())
        .for_update()
        .skip_locked();

    let Schema { users, posts } = Schema::new();
    let _ = builder
        .select(users.id)
        .from(users)
        .union(builder.select(posts.user_id).from(posts));

    let Schema { users, posts } = Schema::new();
    let _ = builder
        .select(alias(users.name, "label"))
        .from(users)
        .union(builder.select(alias(posts.title, "label")).from(posts))
        .order_by(desc(output_alias("label")));

    let Schema { users, posts } = Schema::new();
    expect_inner(builder.select(()).from(users).inner_join(posts));
    let Schema { users, posts } = Schema::new();
    expect_left(builder.select(()).from(users).left_join(posts));
    let Schema { users, posts } = Schema::new();
    expect_right(builder.select(()).from(users).right_join(posts));
}
