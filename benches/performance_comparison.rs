// #![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use divan::{AllocProfiler, Bencher, black_box};
use drizzle_rs::prelude::*;
use rusqlite::Connection;

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
#[divan::bench_group]
mod select {
    use super::*;

    #[divan::bench]
    fn rusqlite(bencher: Bencher) {
        bencher
            .with_inputs(|| {
                let conn = setup_raw_rusqlite();
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
            });
    }

    #[divan::bench]
    fn drizzle_rs(bencher: Bencher) {
        bencher
            .with_inputs(|| {
                let (db, users) = setup_drizzle();
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
    fn drizzle_rs_prepared(bencher: Bencher) {
        bencher
            .with_inputs(|| {
                let (db, users) = setup_drizzle();
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
    fn rusqlite(bencher: Bencher) {
        bencher
            .with_inputs(|| setup_raw_rusqlite())
            .bench_values(|conn| {
                conn.execute(
                    "INSERT INTO users (name, email) VALUES (?1, ?2)",
                    [black_box("user"), black_box("user@example.com")],
                )
                .unwrap()
            });
    }

    #[divan::bench]
    fn drizzle_rs(bencher: Bencher) {
        bencher
            .with_inputs(|| setup_drizzle())
            .bench_values(|(db, user)| {
                db.insert(user)
                    .values([InsertUser::new("user", "user@example.com")])
                    .execute()
                    .unwrap()
            });
    }

    #[divan::bench]
    fn drizzle_rs_prepared(bencher: Bencher) {
        bencher
            .with_inputs(|| {
                let (db, users) = setup_drizzle();
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
    fn rusqlite(bencher: Bencher) {
        bencher
            .with_inputs(|| {
                let conn = setup_raw_rusqlite();

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

    // #[divan::bench]
    // fn drizzle_rs(bencher: Bencher) {
    //     bencher
    //         .with_inputs(|| {
    //             let (db, users) = setup_drizzle();
    //             db.execute(sql!("DROP {users}")).expect("drop users");
    //             db.execute(users.sql()).expect("recreate users table");
    //             let data: Vec<_> = (0..1000)
    //                 .map(|i| {
    //                     InsertUser::new(
    //                         black_box(format!("User {}", i)),
    //                         black_box(format!("user{}@example.com", i)),
    //                     )
    //                 })
    //                 .collect();
    //             (db, users, data)
    //         })
    //         .bench_values(|(db, users, data)| db.insert(users).values(data.iter().map(|f|)).execute().unwrap());
    // }

    #[divan::bench]
    fn drizzle_rs_prepared(bencher: Bencher) {
        bencher
            .with_inputs(|| {
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
            })
            .bench_values(|(db, prepared)| prepared.execute(db.conn(), []).unwrap());
    }
}

fn main() {
    divan::main();
}
