use divan::{AllocProfiler, Bencher, black_box};
use drizzle_rs::prelude::*;
#[cfg(feature = "libsql")]
use libsql::Builder;
#[cfg(feature = "libsql")]
use libsql::Connection;
#[cfg(feature = "rusqlite")]
use rusqlite::Connection;
#[cfg(feature = "turso")]
use turso::Builder;
#[cfg(feature = "turso")]
use turso::Connection;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

// Schema structures for drizzle-rs
#[SQLiteTable(name = "users")]
struct User {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[text]
    email: String,
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
fn setup_raw_connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(CREATE_TABLE_SQL, []).unwrap();
    conn
}
#[cfg(any(feature = "turso", feature = "libsql"))]
async fn setup_raw_connection() -> Connection {
    let db = Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    conn.execute(CREATE_TABLE_SQL, ())
        .await
        .expect("create table");
    conn
}

#[derive(Clone)]
pub struct Schema;
#[cfg(feature = "rusqlite")]
fn setup_drizzle() -> (drizzle_rs::sqlite::Drizzle<Schema>, User) {
    let conn = Connection::open_in_memory().unwrap();
    let (db, users) = drizzle!(conn, User, Schema);
    db.execute(users.sql()).unwrap();

    (db, users)
}

#[cfg(any(feature = "turso", feature = "libsql"))]
async fn setup_drizzle() -> (drizzle_rs::sqlite::Drizzle<Schema>, User) {
    let db = Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in memory");
    let conn = db.connect().expect("connect to db");
    let (db, users) = drizzle!(conn, User, Schema);
    db.execute(users.sql()).await.expect("create table");

    (db, users)
}

#[divan::bench_group]
mod select {
    use super::*;

    #[divan::bench]
    fn rusqlite(bencher: Bencher) {
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

        bencher
            .with_inputs(|| {
                #[cfg(feature = "rusqlite")]
                {
                    for i in 0..100 {
                        conn.execute(
                            "INSERT INTO users (name, email) VALUES (?1, ?2)",
                            [format!("User {}", i), format!("user{}@example.com", i)],
                        )
                        .unwrap();
                    }
                    conn
                }

                #[cfg(any(feature = "turso", feature = "libsql"))]
                rt.block_on(async {
                    let conn = setup_raw_connection().await;
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
                #[cfg(feature = "rusqlite")]
                {
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
                }

                #[cfg(any(feature = "turso", feature = "libsql"))]
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
    fn drizzle_rs(bencher: Bencher) {
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| {
                #[cfg(feature = "rusqlite")]
                {
                    let (db, users) = setup_drizzle();
                    let data = (0..100).map(|i| {
                        InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                    });
                    db.insert(users).values(data).execute().unwrap();
                    (db, users)
                }

                #[cfg(any(feature = "turso", feature = "libsql"))]
                rt.block_on(async {
                    let (db, users) = setup_drizzle().await;
                    let data = (0..100).map(|i| {
                        InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                    });
                    db.insert(users).values(data).execute().await.unwrap();
                    (db, users)
                })
            })
            .bench_values(|(db, users)| {
                #[cfg(feature = "rusqlite")]
                let results: Vec<SelectUser> = db.select(()).from(users).all().unwrap();
                #[cfg(any(feature = "turso", feature = "libsql"))]
                let results: Vec<SelectUser> =
                    rt.block_on(async { db.select(()).from(users).all().await.unwrap() });
                black_box(results);
            });
    }

    #[divan::bench]
    fn drizzle_rs_prepared(bencher: Bencher) {
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| {
                #[cfg(feature = "rusqlite")]
                {
                    let (db, users) = setup_drizzle();
                    let data = (0..100).map(|i| {
                        InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                    });

                    db.insert(users).values(data).execute().unwrap();
                    let prepared = db.select(()).from(users).prepare().into_owned();
                    (db, prepared)
                }
                #[cfg(any(feature = "turso", feature = "libsql"))]
                rt.block_on(async {
                    let (db, users) = setup_drizzle().await;
                    let data = (0..100).map(|i| {
                        InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                    });

                    db.insert(users).values(data).execute().await.unwrap();
                    let prepared = db.select(()).from(users).prepare().into_owned();
                    (db, prepared)
                })
            })
            .bench_values(|(db, prepared)| {
                #[cfg(feature = "rusqlite")]
                let results: Vec<SelectUser> = prepared.all(db.conn(), []).unwrap();
                #[cfg(any(feature = "turso", feature = "libsql"))]
                let results: Vec<SelectUser> =
                    rt.block_on(async { prepared.all(db.conn(), []).await.unwrap() });
                black_box(results);
            });
    }
}

#[divan::bench_group]
mod insert {
    use super::*;

    #[cfg(feature = "rusqlite")]
    #[divan::bench]
    fn rusqlite(bencher: Bencher) {
        bencher
            .with_inputs(|| setup_raw_connection())
            .bench_values(|conn| {
                conn.execute(
                    "INSERT INTO users (name, email) VALUES (?1, ?2)",
                    [black_box("user"), black_box("user@example.com")],
                )
                .unwrap()
            });
    }
    #[cfg(feature = "turso")]
    #[divan::bench]
    fn turso(bencher: Bencher) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| rt.block_on(async { setup_raw_connection().await }))
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

    #[cfg(feature = "libsql")]
    #[divan::bench]
    fn libsql(bencher: Bencher) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| rt.block_on(async { setup_raw_connection().await }))
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
    fn drizzle_rs(bencher: Bencher) {
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| {
                #[cfg(feature = "rusqlite")]
                {
                    setup_drizzle()
                }
                #[cfg(any(feature = "turso", feature = "libsql"))]
                rt.block_on(async { setup_drizzle().await })
            })
            .bench_values(|(db, user)| {
                #[cfg(feature = "rusqlite")]
                db.insert(user)
                    .values([InsertUser::new("user", "user@example.com")])
                    .execute()
                    .unwrap();
                #[cfg(any(feature = "turso", feature = "libsql"))]
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
    fn drizzle_rs_prepared(bencher: Bencher) {
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| {
                #[cfg(feature = "rusqlite")]
                {
                    let (db, users) = setup_drizzle();
                    let prepared = db
                        .insert(users)
                        .values([InsertUser::new("user", "user@example.com")])
                        .prepare()
                        .into_owned();

                    (db, prepared)
                }
                #[cfg(any(feature = "turso", feature = "libsql"))]
                rt.block_on(async {
                    let (db, users) = setup_drizzle().await;
                    let prepared = db
                        .insert(users)
                        .values([InsertUser::new("user", "user@example.com")])
                        .prepare()
                        .into_owned();

                    (db, prepared)
                })
            })
            .bench_values(|(db, prepared)| {
                #[cfg(feature = "rusqlite")]
                prepared.execute(db.conn(), []).unwrap();
                #[cfg(any(feature = "turso", feature = "libsql"))]
                rt.block_on(async {
                    prepared.execute(db.conn(), []).await.unwrap();
                })
            });
    }
}
#[divan::bench_group]
mod bulk_insert {

    use super::*;

    #[cfg(feature = "rusqlite")]
    #[divan::bench]
    fn rusqlite(bencher: Bencher) {
        bencher
            .with_inputs(|| {
                let conn = setup_raw_connection();

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
                conn.execute(&sql, rusqlite::params_from_iter(params))
                    .unwrap();
            });
    }
    #[cfg(feature = "turso")]
    #[divan::bench]
    fn turso(bencher: Bencher) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| {
                rt.block_on(async {
                    let conn = setup_raw_connection().await;

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
                    conn.execute(&sql, turso::params_from_iter(params))
                        .await
                        .unwrap();
                })
            });
    }
    #[cfg(feature = "libsql")]
    #[divan::bench]
    fn libsql(bencher: Bencher) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| {
                rt.block_on(async {
                    let conn = setup_raw_connection().await;

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
                    conn.execute(&sql, libsql::params_from_iter(params))
                        .await
                        .unwrap();
                })
            });
    }

    // #[divan::bench]
    // fn drizzle_rs(bencher: Bencher) {
    //     #[cfg(any(feature = "turso", feature = "libsql"))]
    //     let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    //     bencher
    //         .with_inputs(|| {
    //             #[cfg(feature = "rusqlite")]
    //             {
    //                 let (db, users) = setup_drizzle();
    //                 db.execute(sql!("DROP {users}")).expect("drop users");
    //                 db.execute(users.sql()).expect("recreate users table");
    //                 let data: Vec<_> = (0..1000)
    //                     .map(|i| {
    //                         InsertUser::new(
    //                             black_box(format!("User {}", i)),
    //                             black_box(format!("user{}@example.com", i)),
    //                         )
    //                     })
    //                     .collect();
    //                 (db, users, data)
    //             }
    //             #[cfg(any(feature = "turso", feature = "libsql"))]
    //             rt.block_on(async {
    //                 use procmacros::sql;

    //                 let (db, users) = setup_drizzle().await;
    //                 db.execute(sql!("DROP {users}")).await.expect("drop users");
    //                 db.execute(users.sql()).await.expect("recreate users table");
    //                 let data: Vec<_> = (0..1000)
    //                     .map(|i| {
    //                         InsertUser::new(
    //                             black_box(format!("User {}", i)),
    //                             black_box(format!("user{}@example.com", i)),
    //                         )
    //                     })
    //                     .collect();
    //                 (db, users, data)
    //             })
    //         })
    //         .bench_values(|(db, users, data)| {
    //             #[cfg(feature = "rusqlite")]
    //             db.insert(users).values(data).execute().unwrap();
    //             #[cfg(any(feature = "turso", feature = "libsql"))]
    //             rt.block_on(async { db.insert(users).values(data).execute().await.unwrap() })
    //         });
    // }

    #[divan::bench]
    fn drizzle_rs_prepared(bencher: Bencher) {
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        bencher
            .with_inputs(|| {
                #[cfg(feature = "rusqlite")]
                {
                    let (db, users) = setup_drizzle();

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
                }
                #[cfg(any(feature = "turso", feature = "libsql"))]
                rt.block_on(async {
                    let (db, users) = setup_drizzle().await;

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
                #[cfg(feature = "rusqlite")]
                prepared.execute(db.conn(), []).unwrap();
                #[cfg(any(feature = "turso", feature = "libsql"))]
                rt.block_on(async {
                    prepared.execute(db.conn(), []).await.unwrap();
                })
            })
    }
}

fn main() {
    divan::main();
}
