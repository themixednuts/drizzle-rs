// #![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use drizzle_rs::prelude::*;
use rusqlite::Connection;
use std::hint::black_box;

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

fn setup_raw_rusqlite() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(CREATE_TABLE_SQL, []).unwrap();
    conn
}

#[derive(Clone)]
pub struct Schema;
fn setup_drizzle() -> (drizzle_rs::sqlite::Drizzle<Schema>, User) {
    let conn = Connection::open_in_memory().unwrap();
    let (db, users) = drizzle!(conn, User, Schema);
    db.execute(users.sql()).unwrap();

    (db, users)
}
fn select(c: &mut Criterion) {
    let mut group = c.benchmark_group("select");
    group.bench_function("rusqlite", |b| {
        b.iter_batched(
            || {
                let conn = setup_raw_rusqlite();
                for i in 0..100 {
                    conn.execute(
                        "INSERT INTO users (name, email) VALUES (?1, ?2)",
                        [format!("User {}", i), format!("user{}@example.com", i)],
                    )
                    .unwrap();
                }
                conn
            },
            |conn| {
                let mut stmt = conn
                    .prepare(r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users""#)
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
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("drizzle_rs", |b| {
        b.iter_batched(
            || {
                let (db, users) = setup_drizzle();
                let data = (0..100).map(|i| {
                    InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                });
                db.insert(users).values(data).execute().unwrap();
                (db, users)
            },
            |(db, users)| {
                // Select all data
                let results: Vec<SelectUser> = db.select(()).from(users).all().unwrap();
                black_box(results);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("drizzle_rs_prepared", |b| {
        b.iter_batched(
            || {
                let (db, users) = setup_drizzle();
                // Insert test data
                let data = (0..100).map(|i| {
                    InsertUser::new(format!("User {}", i), format!("user{}@example.com", i))
                });

                db.insert(users).values(data).execute().unwrap();
                // Create prepared select statement
                let prepared = db.select(()).from(users).prepare().into_owned();
                (db, prepared)
            },
            |(db, prepared)| {
                let results: Vec<SelectUser> = prepared.all(db.conn(), []).unwrap();
                black_box(results);
            },
            BatchSize::SmallInput,
        );
    });
}

fn insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    group.bench_function("rusqlite", |b| {
        b.iter_batched(
            || setup_raw_rusqlite(),
            |conn| {
                conn.execute(
                    "INSERT INTO users (name, email) VALUES (?1, ?2)",
                    [black_box("user"), black_box("user@example.com")],
                )
                .unwrap()
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("drizzle_rs", |b| {
        b.iter_batched(
            || setup_drizzle(),
            |(db, user)| {
                db.insert(user)
                    .values([InsertUser::new("user", "user@example.com")])
                    .execute()
                    .unwrap()
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("drizzle_rs_prepared", |b| {
        b.iter_batched(
            || {
                let (db, users) = setup_drizzle();
                // Create a prepared insert statement with placeholder values
                let prepared = db
                    .insert(users)
                    .values([InsertUser::new("user", "user@example.com")])
                    .prepare()
                    .into_owned();

                (db, prepared)
            },
            |(db, prepared)| {
                prepared.execute(db.conn(), []).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
}
fn bulk_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_insert");
    group.bench_function("rusqlite", |b| {
        b.iter_batched(
            || {
                let conn = setup_raw_rusqlite();

                // Build a single INSERT statement with all 1000 rows
                let mut sql = String::from("INSERT INTO users (name, email) VALUES ");
                let mut params: Vec<String> = Vec::with_capacity(2000); // 2 params per row

                for i in 0..1000 {
                    if i > 0 {
                        sql.push_str(", ");
                    }
                    sql.push_str("(?, ?)");
                    params.push(black_box(format!("User {}", i)));
                    params.push(black_box(format!("user{}@example.com", i)));
                }

                (conn, sql, params)
            },
            |(conn, sql, params)| {
                conn.execute(&sql, rusqlite::params_from_iter(params))
                    .unwrap();
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("drizzle_rs", |b| {
        b.iter_batched(
            || {
                let (db, users) = setup_drizzle();

                let data: Vec<_> = (0..1000)
                    .map(|i| {
                        InsertUser::new(
                            black_box(format!("User {}", i)),
                            black_box(format!("user{}@example.com", i)),
                        )
                    })
                    .collect();
                (db, users, data)
            },
            |(db, users, data)| db.insert(users).values(data).execute().unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("drizzle_rs_prepared", |b| {
        b.iter_batched(
            || {
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
            },
            |(db, prepared)| prepared.execute(db.conn(), []).unwrap(),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, select, insert, bulk_insert);
criterion_main!(benches);
