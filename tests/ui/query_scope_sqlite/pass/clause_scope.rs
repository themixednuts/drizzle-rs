use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
    email: Option<String>,
    age: i32,
}

#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i32,
    title: String,
    #[column(references = Users::id)]
    author_id: i32,
    views: i32,
}

#[SQLiteTable]
struct Tags {
    #[column(primary)]
    id: i32,
    #[column(references = Posts::id)]
    post_id: i32,
    label: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    posts: Posts,
    tags: Tags,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { users, posts, tags }) = Drizzle::new(conn);
    let qb = drizzle::sqlite::builder::QueryBuilder::new::<Schema>();
    let _ = (&users, &posts, &tags, &qb);
    // Correlated subqueries read their outer query's sources.
    let has_post = qb
        .select(posts.id)
        .from(posts)
        .r#where(eq(posts.author_id, users.id));
    let _: drizzle::Result<Vec<i32>> =
        db.select(users.id).from(users).r#where(exists(has_post)).all();

    // A subquery over the same table as its outer query resolves to its own copy.
    let adults = qb.select(users.id).from(users).r#where(gt(users.age, 17));
    let _: drizzle::Result<Vec<i32>> = db
        .select(users.id)
        .from(users)
        .r#where(in_subquery(users.id, adults))
        .all();

    // Every clause may read every joined source.
    let _: drizzle::Result<Vec<(i32, String)>> = db
        .select((users.id, posts.title))
        .from(users)
        .join((posts, eq(posts.author_id, users.id)))
        .join((tags, eq(tags.post_id, posts.id)))
        .r#where(eq(tags.label, "rust"))
        .order_by((asc(users.name), desc(posts.views)))
        .all();
    let _: drizzle::Result<Vec<(i32, i64)>> = db
        .select((users.id, count(posts.id)))
        .from(users)
        .join((posts, eq(posts.author_id, users.id)))
        .group_by(users.id)
        .having(gt(count(posts.id), 1))
        .all();

    // COALESCE and COUNT absorb the NULLs an outer join introduces.
    let _: drizzle::Result<Vec<(i32, String)>> = db
        .select((users.id, coalesce(posts.title, "none")))
        .from(users)
        .left_join((posts, eq(posts.author_id, users.id)))
        .all();
    let _: drizzle::Result<Vec<(i32, i64)>> = db
        .select((users.id, count(posts.id)))
        .from(users)
        .left_join((posts, eq(posts.author_id, users.id)))
        .group_by(users.id)
        .all();
    // Nullable results decode as `Option`.
    let _: drizzle::Result<Vec<(i32, Option<i32>, Option<bool>)>> = db
        .select((users.id, posts.views + 1, eq(posts.views, 1)))
        .from(users)
        .left_join((posts, eq(posts.author_id, users.id)))
        .all();

    // Branches of a compound query may filter differently.
    let older = qb.select(users.id).from(users).r#where(gt(users.age, 65));
    let _: drizzle::Result<Vec<i32>> = db
        .select(users.id)
        .from(users)
        .r#where(lt(users.age, 18))
        .union(older)
        .all();

    // Window and CASE expressions over joined sources.
    let _: drizzle::Result<Vec<(i64, Option<i32>)>> = db
        .select((
            row_number().over(window().partition_by([posts.author_id]).order_by(desc(posts.views))),
            case().when(gt(posts.views, 1), users.id).end(),
        ))
        .from(users)
        .join((posts, eq(posts.author_id, users.id)))
        .all();
}
