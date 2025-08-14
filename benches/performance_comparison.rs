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

fn raw_rusqlite_insert(c: &mut Criterion) {
    c.bench_function("rusqlite_insert", |b| {
        b.iter_batched(
            || setup_raw_rusqlite(),
            |conn| {
                for i in 0..100 {
                    conn.execute(
                        "INSERT INTO users (name, email) VALUES (?1, ?2)",
                        [
                            black_box(format!("User {}", i)),
                            black_box(format!("user{}@example.com", i)),
                        ],
                    )
                    .unwrap();
                }
            },
            BatchSize::SmallInput,
        );
    });
}

fn drizzle_rs_insert(c: &mut Criterion) {
    c.bench_function("drizzle_rs_insert", |b| {
        b.iter_batched(
            || {
                let conn = Connection::open_in_memory().unwrap();
                let (db, users) = drizzle!(conn, User);
                db.execute(users.sql()).unwrap();
                (db, users)
            },
            |(db, users)| {
                for i in 0..100 {
                    let data = InsertUser::new(
                        black_box(format!("User {}", i)),
                        black_box(format!("user{}@example.com", i)),
                    );
                    db.insert(users).values([data]).execute().unwrap();
                }
            },
            BatchSize::SmallInput,
        );
    });
}

fn raw_rusqlite_select(c: &mut Criterion) {
    c.bench_function("rusqlite_select", |b| {
        b.iter_batched(
            || {
                let conn = setup_raw_rusqlite();
                // Insert test data
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
                // Select all data
                let mut stmt = conn.prepare("SELECT id, name, email FROM users").unwrap();
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
}

fn drizzle_rs_select(c: &mut Criterion) {
    c.bench_function("drizzle_rs_select", |b| {
        b.iter_batched(
            || {
                let conn = Connection::open_in_memory().unwrap();
                let (db, users) = drizzle!(conn, User);
                db.execute(users.sql()).unwrap();

                // Insert test data
                for i in 0..100 {
                    let data = InsertUser::new(format!("User {}", i), format!("user{}@example.com", i));
                    db.insert(users).values([data]).execute().unwrap();
                }
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
}

fn bulk_operations_comparison(c: &mut Criterion) {
    // Raw rusqlite bulk insert
    c.bench_function("rusqlite_bulk_insert_1000", |b| {
        b.iter_batched(
            || setup_raw_rusqlite(),
            |conn| {
                let mut stmt = conn
                    .prepare("INSERT INTO users (name, email) VALUES (?1, ?2)")
                    .unwrap();

                for i in 0..1000 {
                    stmt.execute([
                        black_box(format!("User {}", i)),
                        black_box(format!("user{}@example.com", i)),
                    ])
                    .unwrap();
                }
            },
            BatchSize::SmallInput,
        );
    });

    // Drizzle-rs bulk insert
    c.bench_function("drizzle_rs_bulk_insert_1000", |b| {
        b.iter_batched(
            || {
                let conn = Connection::open_in_memory().unwrap();
                let (db, users) = drizzle!(conn, User);
                db.execute(users.sql()).unwrap();

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
            |(db, users, data)| {
                db.insert(users).values(data).execute().unwrap();
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    raw_rusqlite_insert,
    drizzle_rs_insert,
    raw_rusqlite_select,
    drizzle_rs_select,
    bulk_operations_comparison
);
criterion_main!(benches);
