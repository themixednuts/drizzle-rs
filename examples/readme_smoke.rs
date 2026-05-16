//! Smoke test for the runnable code samples in `README.md`.
//!
//! Each section below maps to a heading in the README. The goal is to catch
//! type / signature drift the moment the public API changes — if this file
//! stops compiling or asserts fail, the README is wrong.
//!
//! Run with: `cargo run --example readme_smoke --features rusqlite,query`
//!
//! Sections that are intentionally NOT smoke-tested:
//! - Migrations (requires committed files on disk + env vars)
//! - Push (illustrative; would clobber any state)
//! - Type Casting (uses an undefined `json_age` placeholder column)
//! - PostgreSQL (needs a live PG instance)

#[cfg(all(feature = "rusqlite", feature = "query"))]
mod readme {
    // -------- 3. Define Your Schema --------
    use drizzle::sqlite::prelude::*;

    #[SQLiteTable]
    pub struct Users {
        #[column(primary, autoincrement)]
        pub id: i64,
        pub name: String,
        pub email: Option<String>,
        pub age: i64,
    }

    #[SQLiteTable]
    pub struct Posts {
        #[column(primary, autoincrement)]
        pub id: i64,
        pub title: String,
        pub content: Option<String>,
        #[column(references = Users::id)]
        pub author_id: i64,
    }

    #[SQLiteTable]
    pub struct Comments {
        #[column(primary, autoincrement)]
        pub id: i64,
        pub body: String,
        #[column(references = Posts::id)]
        pub post_id: i64,
    }

    #[derive(SQLiteSchema)]
    pub struct Schema {
        pub users: Users,
        pub posts: Posts,
        pub comments: Comments,
    }
}

#[cfg(all(feature = "rusqlite", feature = "query"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use drizzle::core::expr::*;
    use drizzle::error::DrizzleError;
    use drizzle::sqlite::prelude::*;
    use drizzle::sqlite::rusqlite::Drizzle;

    use readme::{
        InsertComments, InsertPosts, InsertUsers, Posts, QueryPostsComments, QueryUsersPosts,
        Schema, SelectUsers, UpdateUsers, Users,
    };

    // -------- 4. Connect & Query --------
    let conn = rusqlite::Connection::open_in_memory()?;
    let (
        mut db,
        Schema {
            users,
            posts,
            comments,
        },
    ) = Drizzle::new(conn, Schema::new());
    db.create()?;

    // Seed enough data so the assertions below have something to chew on.
    db.insert(users)
        .values([
            InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"),
            InsertUsers::new("Jordan Lee", 30i64).with_email("jordan@example.com"),
            InsertUsers::new("Alice", 28i64).with_email("alice@example.com"),
            InsertUsers::new("Bob", 32i64).with_email("bob@example.com"),
        ])
        .execute()?;
    db.insert(posts)
        .values([
            InsertPosts::new("hello", 1i64).with_content("first post"),
            InsertPosts::new("world", 1i64).with_content("second post"),
        ])
        .execute()?;
    db.insert(comments)
        .values([InsertComments::new("nice post", 1i64)])
        .execute()?;

    // -------- Generated Models > Insert (model only, no execute) --------
    let _new_row = InsertUsers::new("Sample Person", 26i64).with_email("sample@example.com");

    // -------- Generated Models > Update (model only) --------
    let _patch = UpdateUsers::default()
        .with_age(27)
        .with_email("new@example.com");

    // -------- Generated Models: SelectUsers populated by full-row select below.
    // (PartialSelectUsers is exercised under Relational Queries > Selecting Specific Columns.)

    // -------- Querying > Select --------
    let all: Vec<SelectUsers> = db.select(()).from(users).all()?;
    assert!(all.len() >= 4);

    let user: SelectUsers = db
        .select(())
        .from(users)
        .r#where(eq(users.name, "Alex Smith"))
        .get()?;
    assert_eq!(user.name, "Alex Smith");

    let names: Vec<(i64, String)> = db.select((users.id, users.name)).from(users).all()?;
    assert_eq!(names.len(), all.len());

    let active_adults: Vec<SelectUsers> = db
        .select(())
        .from(users)
        .r#where(and(gt(users.age, 18), eq(users.name, "Alex Smith")))
        .all()?;
    assert_eq!(active_adults.len(), 1);

    let or_rows: Vec<SelectUsers> = db
        .select(())
        .from(users)
        .r#where(eq(users.name, "Alice") | eq(users.name, "Bob"))
        .all()?;
    assert_eq!(or_rows.len(), 2);

    // -------- Querying > Ordering, Limiting, Pagination --------
    let _page: Vec<SelectUsers> = db
        .select(())
        .from(users)
        .order_by(asc(users.name))
        .limit(10)
        .offset(0)
        .all()?;

    let _multi_sort: Vec<SelectUsers> = db
        .select(())
        .from(users)
        .order_by([asc(users.name), desc(users.age)])
        .all()?;

    // -------- Querying > Group By --------
    let _grouped: Vec<(String, i64)> = db
        .select((users.name, alias(count(users.id), "total")))
        .from(users)
        .group_by(users.name)
        .having(gt(count(users.id), 0))
        .all()?;

    let _multi_group: Vec<(String, i64, i64)> = db
        .select((users.name, users.age, alias(count(users.id), "total")))
        .from(users)
        .group_by((users.name, users.age))
        .all()?;

    // -------- Querying > Insert --------
    db.insert(users)
        .value(InsertUsers::new("Single Person", 40i64).with_email("single@example.com"))
        .execute()?;

    // -------- Querying > Update --------
    db.update(users)
        .set(UpdateUsers::default().with_age(27))
        .r#where(eq(users.id, 1))
        .execute()?;

    // -------- Querying > Delete --------
    db.delete(users).r#where(eq(users.id, 99_999)).execute()?;

    // -------- Querying > Joins --------
    #[allow(dead_code)] // fields read via the trybuild-style row layout, not directly here.
    #[derive(SQLiteFromRow, Debug)]
    #[from(Users)]
    struct UserWithPost {
        #[column(Users::id)]
        user_id: i64,
        name: String,
        // LEFT JOIN — every Posts column must be Option<T> in case the user has no posts.
        #[column(Posts::id)]
        post_id: Option<i64>,
        #[column(Posts::content)]
        content: Option<String>,
    }

    let _explicit: Vec<UserWithPost> = db
        .select(UserWithPost::Select)
        .from(users)
        .left_join((posts, eq(users.id, posts.author_id)))
        .all()?;

    let _auto_fk: Vec<UserWithPost> = db
        .select(UserWithPost::Select)
        .from(users)
        .left_join(posts)
        .all()?;

    // -------- Querying > Subqueries & Set Operations --------
    let min_id = db.select(min(users.id)).from(users);
    let _newer: Vec<SelectUsers> = db
        .select(())
        .from(users)
        .r#where(gt(users.id, min_id))
        .all()?;

    let exact_rows = db
        .select((users.id, users.name))
        .from(users)
        .r#where(eq(users.name, "Alex Smith"));
    let _matched: Vec<SelectUsers> = db
        .select(())
        .from(users)
        .r#where(in_subquery((users.id, users.name), exact_rows))
        .all()?;

    let _set_op: Vec<(String,)> = db
        .select((users.name,))
        .from(users)
        .r#where(lte(users.age, 25))
        .union(
            db.select((users.name,))
                .from(users)
                .r#where(gte(users.age, 30)),
        )
        .order_by(asc(users.name))
        .all()?;

    // -------- Querying > Aliases --------
    tag!(U, "u");
    let u = Users::alias::<U>();
    let _aliased: Vec<(i64,)> = db.select((u.id,)).from(u).all()?;

    // -------- Expressions --------
    let total: (i64,) = db.select((count(users.id),)).from(users).get()?;
    assert!(total.0 >= 4);

    let _oldest: (Option<i64>,) = db.select((max(users.age),)).from(users).get()?;

    let _coalesced: Vec<(String,)> = db
        .select((coalesce(users.email, "unknown"),))
        .from(users)
        .all()?;

    // -------- Relational Queries --------
    let user_rows = db.query(users).with(users.posts()).find_many()?;
    for u in &user_rows {
        let _ = u.posts().len();
    }

    let _found = db
        .query(users)
        .with(users.posts())
        .r#where(eq(users.name, "Alex Smith"))
        .find_first()?;

    let nested = db
        .query(users)
        .with(users.posts().with(posts.comments()))
        .find_many()?;
    if let Some(first) = nested.first() {
        let _ = first.posts().first().map(|p| p.comments().len());
    }

    let _paged = db
        .query(users)
        .with(users.posts())
        .r#where(gt(users.age, 25))
        .order_by(asc(users.name))
        .limit(10)
        .find_many()?;

    // Partial column selection via .columns()
    let partial = db
        .query(users)
        .columns(users.columns().name().email())
        .find_many()?;
    for u in &partial {
        assert!(u.name.is_some());
        assert!(u.id.is_none());
    }

    // Type aliases used in function signatures (no body needed — just verify they exist).
    fn _consume(_: &readme::UsersQueryRow<readme::UsersWithPosts>) {}

    // -------- Transactions --------
    use drizzle::sqlite::connection::SQLiteTransactionType;

    let counted = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        tx.insert(users)
            .value(InsertUsers::new("Tx Person", 28i64))
            .execute()?;
        let rows: Vec<SelectUsers> = tx.select(()).from(users).all()?;
        Ok::<_, DrizzleError>(rows.len())
    })?;
    assert!(counted > 0);

    // Savepoint: outer commits Alice + Bob; the bad inner aborts cleanly.
    let _ = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        tx.insert(users)
            .value(InsertUsers::new("SP Alice", 28i64))
            .execute()?;
        let _ = tx.savepoint(|stx| {
            stx.insert(users)
                .value(InsertUsers::new("SP Bad", -1i64))
                .execute()?;
            Err::<(), _>(DrizzleError::Other("rollback this part".into()))
        });
        tx.insert(users)
            .value(InsertUsers::new("SP Bob", 32i64))
            .execute()?;
        let rows: Vec<SelectUsers> = tx.select(()).from(users).all()?;
        Ok::<_, DrizzleError>(rows.len())
    })?;

    // -------- Prepared Statements --------
    let name_ph = users.name.placeholder("name");
    let find = db
        .select(())
        .from(users)
        .r#where(eq(users.name, name_ph))
        .prepare();
    let _alice: Vec<SelectUsers> = find.all(db.conn(), [name_ph.bind("Alice")])?;
    let _bob: Vec<SelectUsers> = find.all(db.conn(), [name_ph.bind("Bob")])?;

    let new_name = users.name.placeholder("new_name");
    let target = users.id.placeholder("target");
    let stmt = db
        .update(users)
        .set(UpdateUsers::default().with_name(new_name))
        .r#where(eq(users.id, target))
        .prepare();
    stmt.execute(db.conn(), [new_name.bind("New Name"), target.bind(1i64)])?;

    println!("README smoke test passed.");
    Ok(())
}

#[cfg(not(all(feature = "rusqlite", feature = "query")))]
fn main() {
    println!(
        "readme_smoke needs both `rusqlite` and `query` features — try: \
         cargo run --example readme_smoke --features rusqlite,query"
    );
}
