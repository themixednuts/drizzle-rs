#![allow(clippy::redundant_closure)]

use divan::{AllocProfiler, Bencher, black_box};
use drizzle::prelude::*;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

// Schema structures for drizzle
#[SQLiteTable(name = "users")]
struct User {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[text]
    email: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

// Raw SQL schema
const CREATE_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT NOT NULL
    )
"#;

#[cfg(feature = "rusqlite")]
fn setup_rusqlite_connection() -> ::rusqlite::Connection {
    let conn = ::rusqlite::Connection::open_in_memory().unwrap();
    conn.execute(CREATE_TABLE_SQL, []).unwrap();
    conn
}

#[cfg(feature = "turso")]
async fn setup_turso_connection() -> ::turso::Connection {
    let db = ::turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    conn.execute(CREATE_TABLE_SQL, ())
        .await
        .expect("create table");
    conn
}

#[cfg(feature = "libsql")]
async fn setup_libsql_connection() -> ::libsql::Connection {
    let db = ::libsql::Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    conn.execute(CREATE_TABLE_SQL, ())
        .await
        .expect("create table");
    conn
}

#[cfg(feature = "rusqlite")]
fn setup_rusqlite_drizzle() -> (drizzle::rusqlite::Drizzle<Schema>, User) {
    let conn = ::rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user }) = drizzle::rusqlite::Drizzle::new(conn, Schema::new());
    db.create().expect("create tables");

    (db, user)
}

#[cfg(feature = "turso")]
async fn setup_turso_drizzle() -> (drizzle::turso::Drizzle<Schema>, User) {
    let db = ::turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    let (db, schema) = drizzle::turso::Drizzle::new(conn, Schema::new());
    let Schema { user } = schema;

    db.execute(user.sql()).await.expect("create table");

    (db, user)
}

#[cfg(feature = "libsql")]
async fn setup_libsql_drizzle() -> (drizzle::libsql::Drizzle<Schema>, User) {
    let db = ::libsql::Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    let (db, schema) = drizzle::libsql::Drizzle::new(conn, Schema::new());
    let Schema { user } = schema;

    db.execute(user.sql()).await.expect("create table");

    (db, user)
}

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
                    let data = (0..100).map(|i| {
                        InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                    });
                    db.insert(users).values(data).execute().unwrap();
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
                    let data = (0..100).map(|i| {
                        InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                    });

                    db.insert(users).values(data).execute().unwrap();
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
}

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
                        let data = (0..100).map(|i| {
                            InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                        });
                        db.insert(users).values(data).execute().await.unwrap();
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
                        let data = (0..100).map(|i| {
                            InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                        });

                        db.insert(users).values(data).execute().await.unwrap();
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
                        let data = (0..100).map(|i| {
                            InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                        });
                        db.insert(users).values(data).execute().await.unwrap();
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
                        let data = (0..100).map(|i| {
                            InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                        });

                        db.insert(users).values(data).execute().await.unwrap();
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
                        // println!("{}", prepared.to_sql());
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
