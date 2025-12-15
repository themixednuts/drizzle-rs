#![allow(clippy::redundant_closure)]

use divan::{AllocProfiler, Bencher, black_box};
use drizzle::core::{
    SQLSchema,
    expressions::conditions::eq,
    expressions::{alias, count},
};
use drizzle::sqlite::prelude::*;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

// ============================================================================
// Schema Definitions
// ============================================================================

#[SQLiteTable(name = "users")]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
    email: String,
}

#[SQLiteTable(name = "posts")]
struct Post {
    #[column(primary)]
    id: i32,
    title: String,
    content: String,
    author_id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

#[derive(SQLiteSchema)]
struct BlogSchema {
    user: User,
    post: Post,
}

// ============================================================================
// Helper Macros for Reducing Duplication
// ============================================================================

/// Generate data for benchmarks
macro_rules! gen_users {
    ($count:expr) => {
        (0..$count)
            .map(|i| InsertUser::new(format!("User {}", i), format!("user{}@example.com", i)))
    };
}

#[allow(unused_macros)]
macro_rules! gen_posts {
    ($count:expr, $author_id:expr) => {
        (0..$count).map(|i| {
            InsertPost::new(
                format!("Post {}", i),
                format!("Content for post {}", i),
                $author_id,
            )
        })
    };
}

// ============================================================================
// Rusqlite Setup Functions
// ============================================================================

#[cfg(feature = "rusqlite")]
fn setup_rusqlite_connection() -> ::rusqlite::Connection {
    const USER: User = User::new();
    let conn = ::rusqlite::Connection::open_in_memory().unwrap();
    conn.execute(&USER.sql().sql().to_string(), []).unwrap();
    conn
}

#[cfg(feature = "rusqlite")]
fn setup_rusqlite_blog_connection() -> ::rusqlite::Connection {
    const USER: User = User::new();
    const POST: Post = Post::new();
    let conn = ::rusqlite::Connection::open_in_memory().unwrap();
    conn.execute(&USER.sql().sql().to_string(), []).unwrap();
    conn.execute(&POST.sql().sql().to_string(), []).unwrap();
    conn
}

#[cfg(feature = "rusqlite")]
fn setup_rusqlite_drizzle() -> (drizzle::rusqlite::Drizzle<Schema>, User) {
    let conn = ::rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user }) = drizzle::rusqlite::Drizzle::new(conn, Schema::new());
    db.create().expect("create tables");
    (db, user)
}

#[cfg(feature = "rusqlite")]
fn setup_rusqlite_blog_drizzle() -> (drizzle::rusqlite::Drizzle<BlogSchema>, User, Post) {
    let conn = ::rusqlite::Connection::open_in_memory().unwrap();
    let (db, BlogSchema { user, post }) = drizzle::rusqlite::Drizzle::new(conn, BlogSchema::new());
    db.create().expect("create tables");
    (db, user, post)
}

// ============================================================================
// Turso Setup Functions
// ============================================================================

#[cfg(feature = "turso")]
async fn setup_turso_connection() -> ::turso::Connection {
    const USER: User = User::new();
    let db = ::turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    conn.execute(&USER.sql().sql().to_string(), ())
        .await
        .expect("create table");
    conn
}

#[cfg(feature = "turso")]
async fn setup_turso_drizzle() -> (drizzle::turso::Drizzle<Schema>, User) {
    let db = ::turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    let (db, Schema { user }) = drizzle::turso::Drizzle::new(conn, Schema::new());
    db.execute(user.sql()).await.expect("create table");
    (db, user)
}

// ============================================================================
// Libsql Setup Functions
// ============================================================================

#[cfg(feature = "libsql")]
async fn setup_libsql_connection() -> ::libsql::Connection {
    const USER: User = User::new();
    let db = ::libsql::Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    conn.execute(&USER.sql().sql().to_string(), ())
        .await
        .expect("create table");
    conn
}

#[cfg(feature = "libsql")]
async fn setup_libsql_drizzle() -> (drizzle::libsql::Drizzle<Schema>, User) {
    let db = ::libsql::Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    let (db, Schema { user }) = drizzle::libsql::Drizzle::new(conn, Schema::new());
    db.execute(user.sql()).await.expect("create table");
    (db, user)
}

// ============================================================================
// Rusqlite Benchmarks
// ============================================================================

#[cfg(feature = "rusqlite")]
#[divan::bench_group]
mod rusqlite {
    use super::*;

    #[divan::bench_group]
    mod select {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let conn = setup_rusqlite_connection();
                    for i in 0..100 {
                        conn.execute(
                            "INSERT INTO users (name, email) VALUES (?1, ?2)",
                            [format!("User {}", i), format!("user{}@example.com", i)],
                        )
                        .unwrap();
                    }
                    conn
                })
                .bench_values(|conn| {
                    let mut stmt = conn
                        .prepare(
                            r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users""#,
                        )
                        .unwrap();

                    let rows = stmt
                        .query_map([], |row| {
                            Ok((
                                row.get::<_, i32>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                            ))
                        })
                        .unwrap();

                    let results: Vec<_> = rows.collect();
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    db.insert(users).values(gen_users!(100)).execute().unwrap();
                    (db, users)
                })
                .bench_values(|(db, users)| {
                    let results: Vec<SelectUser> = db.select(()).from(users).all().unwrap();
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    db.insert(users).values(gen_users!(100)).execute().unwrap();
                    let prepared = db.select(()).from(users).prepare().into_owned();
                    (db, prepared)
                })
                .bench_values(|(db, prepared)| {
                    let results: Vec<SelectUser> = prepared.all(db.conn(), []).unwrap();
                    black_box(results);
                });
        }
    }

    #[divan::bench_group]
    mod select_where {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let conn = setup_rusqlite_connection();
                    for i in 0..100 {
                        conn.execute(
                            "INSERT INTO users (name, email) VALUES (?1, ?2)",
                            [format!("User {}", i), format!("user{}@example.com", i)],
                        )
                        .unwrap();
                    }
                    conn
                })
                .bench_values(|conn| {
                    let mut stmt = conn
                        .prepare(
                            r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users" WHERE "users"."id" = ?1"#,
                        )
                        .unwrap();

                    let rows = stmt
                        .query_map([black_box(50)], |row| {
                            Ok((
                                row.get::<_, i32>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                            ))
                        })
                        .unwrap();

                    let results: Vec<_> = rows.collect();
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    db.insert(users).values(gen_users!(100)).execute().unwrap();
                    (db, users)
                })
                .bench_values(|(db, users)| {
                    let results: Vec<SelectUser> = db
                        .select(())
                        .from(users)
                        .r#where(eq(users.id, black_box(50)))
                        .all()
                        .unwrap();
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    db.insert(users).values(gen_users!(100)).execute().unwrap();
                    let prepared = db
                        .select(())
                        .from(users)
                        .r#where(eq(users.id, 50))
                        .prepare()
                        .into_owned();
                    (db, prepared)
                })
                .bench_values(|(db, prepared)| {
                    let results: Vec<SelectUser> = prepared.all(db.conn(), []).unwrap();
                    black_box(results);
                });
        }
    }

    #[divan::bench_group]
    mod insert {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            bencher
                .with_inputs(|| setup_rusqlite_connection())
                .bench_values(|conn| {
                    conn.execute(
                        "INSERT INTO users (name, email) VALUES (?1, ?2)",
                        [black_box("user"), black_box("user@example.com")],
                    )
                    .unwrap()
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| setup_rusqlite_drizzle())
                .bench_values(|(db, user)| {
                    db.insert(user)
                        .values([InsertUser::new("user", "user@example.com")])
                        .execute()
                        .unwrap();
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    let prepared = db
                        .insert(users)
                        .values([InsertUser::new("user", "user@example.com")])
                        .prepare()
                        .into_owned();

                    (db, prepared)
                })
                .bench_values(|(db, prepared)| {
                    prepared.execute(db.conn(), []).unwrap();
                });
        }
    }

    #[divan::bench_group]
    mod update {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let conn = setup_rusqlite_connection();
                    conn.execute(
                        "INSERT INTO users (id, name, email) VALUES (1, 'user', 'user@example.com')",
                        [],
                    )
                    .unwrap();
                    conn
                })
                .bench_values(|conn| {
                    conn.execute(
                        r#"UPDATE "users" SET "name" = ?1 WHERE "users"."id" = ?2"#,
                        [black_box("updated"), black_box("1")],
                    )
                    .unwrap()
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    db.insert(users)
                        .values([InsertUser::new("user", "user@example.com")])
                        .execute()
                        .unwrap();
                    (db, users)
                })
                .bench_values(|(db, users)| {
                    db.update(users)
                        .set(UpdateUser::default().with_name(black_box("updated")))
                        .r#where(eq(users.id, black_box(1)))
                        .execute()
                        .unwrap();
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    db.insert(users)
                        .values([InsertUser::new("user", "user@example.com")])
                        .execute()
                        .unwrap();
                    let prepared = db
                        .update(users)
                        .set(UpdateUser::default().with_name("updated"))
                        .r#where(eq(users.id, 1))
                        .prepare()
                        .into_owned();
                    (db, prepared)
                })
                .bench_values(|(db, prepared)| {
                    prepared.execute(db.conn(), []).unwrap();
                });
        }
    }

    #[divan::bench_group]
    mod delete {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let conn = setup_rusqlite_connection();
                    for i in 0..10 {
                        conn.execute(
                            "INSERT INTO users (name, email) VALUES (?1, ?2)",
                            [format!("User {}", i), format!("user{}@example.com", i)],
                        )
                        .unwrap();
                    }
                    conn
                })
                .bench_values(|conn| {
                    conn.execute(
                        r#"DELETE FROM "users" WHERE "users"."id" = ?1"#,
                        [black_box(1)],
                    )
                    .unwrap()
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    db.insert(users).values(gen_users!(10)).execute().unwrap();
                    (db, users)
                })
                .bench_values(|(db, users)| {
                    db.delete(users)
                        .r#where(eq(users.id, black_box(1)))
                        .execute()
                        .unwrap();
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    db.insert(users).values(gen_users!(10)).execute().unwrap();
                    let prepared = db
                        .delete(users)
                        .r#where(eq(users.id, 1))
                        .prepare()
                        .into_owned();
                    (db, prepared)
                })
                .bench_values(|(db, prepared)| {
                    prepared.execute(db.conn(), []).unwrap();
                });
        }
    }

    #[divan::bench_group]
    mod bulk_insert {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let conn = setup_rusqlite_connection();

                    let mut sql = String::from("INSERT INTO users (name, email) VALUES ");
                    let mut params: Vec<String> = Vec::with_capacity(2000);

                    for i in 0..1000 {
                        if i > 0 {
                            sql.push_str(", ");
                        }
                        sql.push_str("(?, ?)");
                        params.push(black_box(format!("User {}", i)));
                        params.push(black_box(format!("user{}@example.com", i)));
                    }

                    (conn, sql, params)
                })
                .bench_values(|(conn, sql, params)| {
                    conn.execute(&sql, ::rusqlite::params_from_iter(params))
                        .unwrap();
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();
                    let data: Vec<_> = (0..1000)
                        .map(|i| {
                            InsertUser::new(
                                black_box(format!("User {}", i)),
                                black_box(format!("user{}@example.com", i)),
                            )
                        })
                        .collect();
                    (db, users, data)
                })
                .bench_values(|(db, users, data)| {
                    db.insert(users).values(data).execute().unwrap();
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_rusqlite_drizzle();

                    let data: Vec<_> = (0..1000)
                        .map(|i| {
                            InsertUser::new(
                                black_box(format!("User {}", i)),
                                black_box(format!("user{}@example.com", i)),
                            )
                        })
                        .collect();
                    let stmt = db.insert(users).values(data);
                    let prepared = stmt.prepare().into_owned();
                    (db, prepared)
                })
                .bench_values(|(db, prepared)| {
                    prepared.execute(db.conn(), []).unwrap();
                })
        }
    }

    #[divan::bench_group]
    mod complex {
        use super::*;

        mod join {
            use super::*;

            #[derive(Debug, SQLiteFromRow)]
            #[allow(dead_code)]
            struct JoinResult {
                #[column(User::name)]
                user_name: String,
                #[column(Post::title)]
                post_title: String,
            }

            impl Default for JoinResult {
                fn default() -> Self {
                    Self {
                        user_name: String::new(),
                        post_title: String::new(),
                    }
                }
            }

            #[divan::bench]
            fn raw(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let conn = setup_rusqlite_blog_connection();
                        // Insert users
                        for i in 0..10 {
                            conn.execute(
                                "INSERT INTO users (id, name, email) VALUES (?1, ?2, ?3)",
                                (
                                    i + 1,
                                    format!("User {}", i),
                                    format!("user{}@example.com", i),
                                ),
                            )
                            .unwrap();
                        }
                        // Insert posts
                        for i in 0..100 {
                            conn.execute(
                                "INSERT INTO posts (title, content, author_id) VALUES (?1, ?2, ?3)",
                                (
                                    format!("Post {}", i),
                                    format!("Content {}", i),
                                    (i % 10) + 1,
                                ),
                            )
                            .unwrap();
                        }
                        conn
                    })
                    .bench_values(|conn| {
                        let mut stmt = conn
                            .prepare(
                                r#"SELECT "users"."name", "posts"."title" FROM "users" 
                                   INNER JOIN "posts" ON "users"."id" = "posts"."author_id""#,
                            )
                            .unwrap();

                        let rows = stmt
                            .query_map([], |row| {
                                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                            })
                            .unwrap();

                        let results: Vec<_> = rows.collect();
                        black_box(results);
                    });
            }

            #[divan::bench]
            fn drizzle(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let (db, users, posts) = setup_rusqlite_blog_drizzle();
                        // Insert users
                        db.insert(users).values(gen_users!(10)).execute().unwrap();
                        // Insert posts
                        let post_data: Vec<_> = (0..100)
                            .map(|i| {
                                InsertPost::new(
                                    format!("Post {}", i),
                                    format!("Content {}", i),
                                    (i % 10) + 1,
                                )
                            })
                            .collect();
                        db.insert(posts).values(post_data).execute().unwrap();
                        (db, users, posts)
                    })
                    .bench_values(|(db, users, posts)| {
                        let results: Vec<JoinResult> = db
                            .select(JoinResult::default())
                            .from(users)
                            .inner_join(posts, eq(users.id, posts.author_id))
                            .all()
                            .unwrap();
                        black_box(results);
                    });
            }
        }

        mod aggregate {
            use super::*;

            #[derive(Debug, SQLiteFromRow)]
            #[allow(dead_code)]
            struct CountResult {
                count: i32,
            }

            #[divan::bench]
            fn raw(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let conn = setup_rusqlite_connection();
                        for i in 0..100 {
                            conn.execute(
                                "INSERT INTO users (name, email) VALUES (?1, ?2)",
                                [format!("User {}", i), format!("user{}@example.com", i)],
                            )
                            .unwrap();
                        }
                        conn
                    })
                    .bench_values(|conn| {
                        let count: i32 = conn
                            .query_row(r#"SELECT COUNT(*) FROM "users""#, [], |row| row.get(0))
                            .unwrap();
                        black_box(count);
                    });
            }

            #[divan::bench]
            fn drizzle(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let (db, users) = setup_rusqlite_drizzle();
                        db.insert(users).values(gen_users!(100)).execute().unwrap();
                        (db, users)
                    })
                    .bench_values(|(db, users)| {
                        let results: Vec<CountResult> = db
                            .select(alias(count(users.id), "count"))
                            .from(users)
                            .all()
                            .unwrap();
                        black_box(results);
                    });
            }
        }

        mod order_limit {
            use super::*;
            use drizzle_core::OrderBy;

            #[divan::bench]
            fn raw(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let conn = setup_rusqlite_connection();
                        for i in 0..100 {
                            conn.execute(
                                "INSERT INTO users (name, email) VALUES (?1, ?2)",
                                [format!("User {}", i), format!("user{}@example.com", i)],
                            )
                            .unwrap();
                        }
                        conn
                    })
                    .bench_values(|conn| {
                        let mut stmt = conn
                            .prepare(
                                r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users" 
                                   ORDER BY "users"."name" ASC LIMIT 10 OFFSET 20"#,
                            )
                            .unwrap();

                        let rows = stmt
                            .query_map([], |row| {
                                Ok((
                                    row.get::<_, i32>(0)?,
                                    row.get::<_, String>(1)?,
                                    row.get::<_, String>(2)?,
                                ))
                            })
                            .unwrap();

                        let results: Vec<_> = rows.collect();
                        black_box(results);
                    });
            }

            #[divan::bench]
            fn drizzle(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let (db, users) = setup_rusqlite_drizzle();
                        db.insert(users).values(gen_users!(100)).execute().unwrap();
                        (db, users)
                    })
                    .bench_values(|(db, users)| {
                        let results: Vec<SelectUser> = db
                            .select(())
                            .from(users)
                            .order_by([OrderBy::asc(users.name)])
                            .limit(10)
                            .offset(20)
                            .all()
                            .unwrap();
                        black_box(results);
                    });
            }
        }
    }
}

// ============================================================================
// Turso Benchmarks
// ============================================================================

#[cfg(feature = "turso")]
#[divan::bench_group]
mod turso {
    use super::*;

    #[divan::bench_group]
    mod select {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let conn = setup_turso_connection().await;
                        for i in 0..100 {
                            conn.execute(
                                "INSERT INTO users (name, email) VALUES (?1, ?2)",
                                [format!("User {}", i), format!("user{}@example.com", i)],
                            )
                            .await
                            .unwrap();
                        }
                        conn
                    })
                })
                .bench_values(|conn| {
                    rt.block_on(async {
                        let mut stmt = conn
                            .prepare(
                                r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users""#,
                            )
                            .await
                            .unwrap();

                        let mut rows = stmt.query(()).await.expect("query rows");

                        let mut results = Vec::new();
                        while let Some(row) = rows.next().await.expect("get row") {
                            let col0: i32 = *row.get_value(0).unwrap().as_integer().unwrap() as i32;
                            let col1: String = row.get_value(1).unwrap().as_text().unwrap().to_string();
                            let col2: String = row.get_value(2).unwrap().as_text().unwrap().to_string();

                            results.push((col0, col1, col2));
                        }

                        black_box(results);
                    });
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;
                        db.insert(users)
                            .values(gen_users!(100))
                            .execute()
                            .await
                            .unwrap();
                        (db, users)
                    })
                })
                .bench_values(|(db, users)| {
                    let results: Vec<SelectUser> =
                        rt.block_on(async { db.select(()).from(users).all().await.unwrap() });
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;
                        db.insert(users)
                            .values(gen_users!(100))
                            .execute()
                            .await
                            .unwrap();
                        let prepared = db.select(()).from(users).prepare().into_owned();
                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    let results: Vec<SelectUser> =
                        rt.block_on(async { prepared.all(db.conn(), []).await.unwrap() });
                    black_box(results);
                });
        }
    }

    #[divan::bench_group]
    mod insert {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| rt.block_on(async { setup_turso_connection().await }))
                .bench_values(|conn| {
                    rt.block_on(async {
                        conn.execute(
                            "INSERT INTO users (name, email) VALUES (?1, ?2)",
                            [black_box("user"), black_box("user@example.com")],
                        )
                        .await
                        .unwrap()
                    })
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| rt.block_on(async { setup_turso_drizzle().await }))
                .bench_values(|(db, user)| {
                    rt.block_on(async move {
                        db.insert(user)
                            .values([InsertUser::new("user", "user@example.com")])
                            .execute()
                            .await
                            .unwrap()
                    })
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;
                        let prepared = db
                            .insert(users)
                            .values([InsertUser::new("user", "user@example.com")])
                            .prepare()
                            .into_owned();

                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    rt.block_on(async {
                        prepared.execute(db.conn(), []).await.unwrap();
                    })
                });
        }
    }

    #[divan::bench_group]
    mod update {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let conn = setup_turso_connection().await;
                        conn.execute(
                            "INSERT INTO users (id, name, email) VALUES (1, 'user', 'user@example.com')",
                            (),
                        )
                        .await
                        .unwrap();
                        conn
                    })
                })
                .bench_values(|conn| {
                    rt.block_on(async {
                        conn.execute(
                            r#"UPDATE "users" SET "name" = ?1 WHERE "users"."id" = ?2"#,
                            [black_box("updated"), black_box("1")],
                        )
                        .await
                        .unwrap()
                    })
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;
                        db.insert(users)
                            .values([InsertUser::new("user", "user@example.com")])
                            .execute()
                            .await
                            .unwrap();
                        (db, users)
                    })
                })
                .bench_values(|(db, users)| {
                    rt.block_on(async {
                        db.update(users)
                            .set(UpdateUser::default().with_name(black_box("updated")))
                            .r#where(eq(users.id, black_box(1)))
                            .execute()
                            .await
                            .unwrap();
                    })
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;
                        db.insert(users)
                            .values([InsertUser::new("user", "user@example.com")])
                            .execute()
                            .await
                            .unwrap();
                        let prepared = db
                            .update(users)
                            .set(UpdateUser::default().with_name("updated"))
                            .r#where(eq(users.id, 1))
                            .prepare()
                            .into_owned();
                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    rt.block_on(async {
                        prepared.execute(db.conn(), []).await.unwrap();
                    })
                });
        }
    }

    #[divan::bench_group]
    mod delete {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let conn = setup_turso_connection().await;
                        for i in 0..10 {
                            conn.execute(
                                "INSERT INTO users (name, email) VALUES (?1, ?2)",
                                [format!("User {}", i), format!("user{}@example.com", i)],
                            )
                            .await
                            .unwrap();
                        }
                        conn
                    })
                })
                .bench_values(|conn| {
                    rt.block_on(async {
                        conn.execute(
                            r#"DELETE FROM "users" WHERE "users"."id" = ?1"#,
                            [black_box(1)],
                        )
                        .await
                        .unwrap()
                    })
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;
                        db.insert(users)
                            .values(gen_users!(10))
                            .execute()
                            .await
                            .unwrap();
                        (db, users)
                    })
                })
                .bench_values(|(db, users)| {
                    rt.block_on(async {
                        db.delete(users)
                            .r#where(eq(users.id, black_box(1)))
                            .execute()
                            .await
                            .unwrap();
                    })
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;
                        db.insert(users)
                            .values(gen_users!(10))
                            .execute()
                            .await
                            .unwrap();
                        let prepared = db
                            .delete(users)
                            .r#where(eq(users.id, 1))
                            .prepare()
                            .into_owned();
                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    rt.block_on(async {
                        prepared.execute(db.conn(), []).await.unwrap();
                    })
                });
        }
    }

    #[divan::bench_group]
    mod bulk_insert {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let conn = setup_turso_connection().await;

                        let mut sql = String::from("INSERT INTO users (name, email) VALUES ");
                        let mut params: Vec<String> = Vec::with_capacity(2000);

                        for i in 0..1000 {
                            if i > 0 {
                                sql.push_str(", ");
                            }
                            sql.push_str("(?, ?)");
                            params.push(black_box(format!("User {}", i)));
                            params.push(black_box(format!("user{}@example.com", i)));
                        }

                        (conn, sql, params)
                    })
                })
                .bench_values(|(conn, sql, params)| {
                    rt.block_on(async {
                        conn.execute(&sql, ::turso::params_from_iter(params))
                            .await
                            .unwrap();
                    })
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;
                        let data: Vec<_> = (0..1000)
                            .map(|i| {
                                InsertUser::new(
                                    black_box(format!("User {}", i)),
                                    black_box(format!("user{}@example.com", i)),
                                )
                            })
                            .collect();
                        (db, users, data)
                    })
                })
                .bench_values(|(db, users, data)| {
                    rt.block_on(async {
                        db.insert(users).values(data).execute().await.unwrap();
                    })
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_turso_drizzle().await;

                        let data: Vec<_> = (0..1000)
                            .map(|i| {
                                InsertUser::new(
                                    black_box(format!("User {}", i)),
                                    black_box(format!("user{}@example.com", i)),
                                )
                            })
                            .collect();
                        let prepared = db.insert(users).values(data).prepare().into_owned();
                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    rt.block_on(async {
                        prepared.execute(db.conn(), []).await.unwrap();
                    })
                })
        }
    }
}

// ============================================================================
// Libsql Benchmarks
// ============================================================================

#[cfg(feature = "libsql")]
#[divan::bench_group]
mod libsql {
    use super::*;

    #[divan::bench_group]
    mod select {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let conn = setup_libsql_connection().await;
                        for i in 0..100 {
                            conn.execute(
                                "INSERT INTO users (name, email) VALUES (?1, ?2)",
                                [format!("User {}", i), format!("user{}@example.com", i)],
                            )
                            .await
                            .unwrap();
                        }
                        conn
                    })
                })
                .bench_values(|conn| {
                    rt.block_on(async {
                        let stmt = conn
                            .prepare(
                                r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users""#,
                            )
                            .await
                            .unwrap();

                        let mut rows = stmt.query(()).await.expect("query rows");

                        let mut results = Vec::new();
                        while let Some(row) = rows.next().await.expect("get row") {
                            let col0: i32 = row.get(0).unwrap();
                            let col1: String = row.get(1).unwrap();
                            let col2: String = row.get(2).unwrap();

                            results.push((col0, col1, col2));
                        }

                        black_box(results);
                    });
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;
                        db.insert(users)
                            .values(gen_users!(100))
                            .execute()
                            .await
                            .unwrap();
                        (db, users)
                    })
                })
                .bench_values(|(db, users)| {
                    let results: Vec<SelectUser> =
                        rt.block_on(async { db.select(()).from(users).all().await.unwrap() });
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;
                        db.insert(users)
                            .values(gen_users!(100))
                            .execute()
                            .await
                            .unwrap();
                        let prepared = db.select(()).from(users).prepare().into_owned();
                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    let results: Vec<SelectUser> =
                        rt.block_on(async { prepared.all(db.conn(), []).await.unwrap() });
                    black_box(results);
                });
        }
    }

    #[divan::bench_group]
    mod insert {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| rt.block_on(async { setup_libsql_connection().await }))
                .bench_values(|conn| {
                    rt.block_on(async {
                        conn.execute(
                            "INSERT INTO users (name, email) VALUES (?1, ?2)",
                            [black_box("user"), black_box("user@example.com")],
                        )
                        .await
                        .unwrap()
                    })
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| rt.block_on(async { setup_libsql_drizzle().await }))
                .bench_values(|(db, user)| {
                    rt.block_on(async move {
                        db.insert(user)
                            .values([InsertUser::new("user", "user@example.com")])
                            .execute()
                            .await
                            .unwrap()
                    })
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;
                        let prepared = db
                            .insert(users)
                            .values([InsertUser::new("user", "user@example.com")])
                            .prepare()
                            .into_owned();

                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    rt.block_on(async {
                        prepared.execute(db.conn(), []).await.unwrap();
                    })
                });
        }
    }

    #[divan::bench_group]
    mod update {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let conn = setup_libsql_connection().await;
                        conn.execute(
                            "INSERT INTO users (id, name, email) VALUES (1, 'user', 'user@example.com')",
                            (),
                        )
                        .await
                        .unwrap();
                        conn
                    })
                })
                .bench_values(|conn| {
                    rt.block_on(async {
                        conn.execute(
                            r#"UPDATE "users" SET "name" = ?1 WHERE "users"."id" = ?2"#,
                            [black_box("updated"), black_box("1")],
                        )
                        .await
                        .unwrap()
                    })
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;
                        db.insert(users)
                            .values([InsertUser::new("user", "user@example.com")])
                            .execute()
                            .await
                            .unwrap();
                        (db, users)
                    })
                })
                .bench_values(|(db, users)| {
                    rt.block_on(async {
                        db.update(users)
                            .set(UpdateUser::default().with_name(black_box("updated")))
                            .r#where(eq(users.id, black_box(1)))
                            .execute()
                            .await
                            .unwrap();
                    })
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;
                        db.insert(users)
                            .values([InsertUser::new("user", "user@example.com")])
                            .execute()
                            .await
                            .unwrap();
                        let prepared = db
                            .update(users)
                            .set(UpdateUser::default().with_name("updated"))
                            .r#where(eq(users.id, 1))
                            .prepare()
                            .into_owned();
                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    rt.block_on(async {
                        prepared.execute(db.conn(), []).await.unwrap();
                    })
                });
        }
    }

    #[divan::bench_group]
    mod delete {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let conn = setup_libsql_connection().await;
                        for i in 0..10 {
                            conn.execute(
                                "INSERT INTO users (name, email) VALUES (?1, ?2)",
                                [format!("User {}", i), format!("user{}@example.com", i)],
                            )
                            .await
                            .unwrap();
                        }
                        conn
                    })
                })
                .bench_values(|conn| {
                    rt.block_on(async {
                        conn.execute(
                            r#"DELETE FROM "users" WHERE "users"."id" = ?1"#,
                            [black_box(1)],
                        )
                        .await
                        .unwrap()
                    })
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;
                        db.insert(users)
                            .values(gen_users!(10))
                            .execute()
                            .await
                            .unwrap();
                        (db, users)
                    })
                })
                .bench_values(|(db, users)| {
                    rt.block_on(async {
                        db.delete(users)
                            .r#where(eq(users.id, black_box(1)))
                            .execute()
                            .await
                            .unwrap();
                    })
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;
                        db.insert(users)
                            .values(gen_users!(10))
                            .execute()
                            .await
                            .unwrap();
                        let prepared = db
                            .delete(users)
                            .r#where(eq(users.id, 1))
                            .prepare()
                            .into_owned();
                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    rt.block_on(async {
                        prepared.execute(db.conn(), []).await.unwrap();
                    })
                });
        }
    }

    #[divan::bench_group]
    mod bulk_insert {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let conn = setup_libsql_connection().await;

                        let mut sql = String::from("INSERT INTO users (name, email) VALUES ");
                        let mut params: Vec<String> = Vec::with_capacity(2000);

                        for i in 0..1000 {
                            if i > 0 {
                                sql.push_str(", ");
                            }
                            sql.push_str("(?, ?)");
                            params.push(black_box(format!("User {}", i)));
                            params.push(black_box(format!("user{}@example.com", i)));
                        }

                        (conn, sql, params)
                    })
                })
                .bench_values(|(conn, sql, params)| {
                    rt.block_on(async {
                        conn.execute(&sql, ::libsql::params_from_iter(params))
                            .await
                            .unwrap();
                    })
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;
                        let data: Vec<_> = (0..1000)
                            .map(|i| {
                                InsertUser::new(
                                    black_box(format!("User {}", i)),
                                    black_box(format!("user{}@example.com", i)),
                                )
                            })
                            .collect();
                        (db, users, data)
                    })
                })
                .bench_values(|(db, users, data)| {
                    rt.block_on(async {
                        db.insert(users).values(data).execute().await.unwrap();
                    })
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            bencher
                .with_inputs(|| {
                    rt.block_on(async {
                        let (db, users) = setup_libsql_drizzle().await;

                        let data: Vec<_> = (0..1000)
                            .map(|i| {
                                InsertUser::new(
                                    black_box(format!("User {}", i)),
                                    black_box(format!("user{}@example.com", i)),
                                )
                            })
                            .collect();
                        let prepared = db.insert(users).values(data).prepare().into_owned();
                        (db, prepared)
                    })
                })
                .bench_values(|(db, prepared)| {
                    rt.block_on(async {
                        prepared.execute(db.conn(), []).await.unwrap();
                    })
                })
        }
    }
}

fn main() {
    divan::main();
}
