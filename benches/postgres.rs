#![allow(clippy::redundant_closure)]

use divan::{AllocProfiler, Bencher, black_box};
use drizzle::core::SQLSchema;
use drizzle::core::expr::{alias, count, eq};
use drizzle::postgres::prelude::*;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

// ============================================================================
// Schema Definitions
// ============================================================================

#[PostgresTable(name = "bench_users")]
struct User {
    #[column(serial, primary)]
    id: i32,
    name: String,
    email: String,
}

#[PostgresTable(name = "bench_posts")]
struct Post {
    #[column(serial, primary)]
    id: i32,
    title: String,
    content: String,
    author_id: i32,
}

#[derive(PostgresSchema)]
struct Schema {
    user: User,
}

#[derive(PostgresSchema)]
struct BlogSchema {
    user: User,
    post: Post,
}

// ============================================================================
// Helper Macros
// ============================================================================

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
// Setup Functions
// ============================================================================

fn get_database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
    })
}

#[cfg(feature = "postgres-sync")]
fn setup_postgres_connection() -> ::postgres::Client {
    const USER: User = User::new();
    let mut client = ::postgres::Client::connect(&get_database_url(), ::postgres::NoTls).expect(
        "Failed to connect to PostgreSQL - is Docker running? (docker compose up -d postgres)",
    );
    client
        .batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .unwrap();
    client.batch_execute(&USER.sql().sql().to_string()).unwrap();
    client
}

#[cfg(feature = "postgres-sync")]
fn setup_postgres_blog_connection() -> ::postgres::Client {
    const USER: User = User::new();
    const POST: Post = Post::new();
    let mut client = ::postgres::Client::connect(&get_database_url(), ::postgres::NoTls)
        .expect("Failed to connect to PostgreSQL");
    client
        .batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .unwrap();
    client.batch_execute(&USER.sql().sql().to_string()).unwrap();
    client.batch_execute(&POST.sql().sql().to_string()).unwrap();
    client
}

#[cfg(feature = "postgres-sync")]
fn setup_postgres_drizzle() -> (drizzle::postgres::sync::Drizzle<Schema>, User) {
    let mut client = ::postgres::Client::connect(&get_database_url(), ::postgres::NoTls)
        .expect("Failed to connect to PostgreSQL");
    client
        .batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .unwrap();
    let (mut db, Schema { user }) = drizzle::postgres::sync::Drizzle::new(client, Schema::new());
    db.create().expect("create tables");
    (db, user)
}

#[cfg(feature = "postgres-sync")]
fn seed_users_fast(db: &mut drizzle::postgres::sync::Drizzle<Schema>, count: i32) {
    db.mut_client()
        .execute(
            "INSERT INTO bench_users (name, email) SELECT 'User ' || g::text, 'user' || g::text || '@example.com' FROM generate_series(0, $1) AS g",
            &[&(count - 1)],
        )
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
fn seed_one_user_fast(db: &mut drizzle::postgres::sync::Drizzle<Schema>) {
    db.mut_client()
        .execute(
            "INSERT INTO bench_users (name, email) VALUES ('user', 'user@example.com')",
            &[],
        )
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
fn setup_postgres_blog_drizzle() -> (drizzle::postgres::sync::Drizzle<BlogSchema>, User, Post) {
    let mut client = ::postgres::Client::connect(&get_database_url(), ::postgres::NoTls)
        .expect("Failed to connect to PostgreSQL");
    client
        .batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .unwrap();
    let (mut db, BlogSchema { user, post }) =
        drizzle::postgres::sync::Drizzle::new(client, BlogSchema::new());
    db.create().expect("create tables");
    (db, user, post)
}

// ============================================================================
// PostgreSQL Sync Benchmarks
// ============================================================================

#[cfg(feature = "postgres-sync")]
#[divan::bench_group]
mod postgres_sync {
    use super::*;

    #[divan::bench_group]
    mod select {
        use super::*;

        #[divan::bench]
        fn raw(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let mut client = setup_postgres_connection();
                    for i in 0..100 {
                        client
                            .execute(
                                "INSERT INTO bench_users (name, email) VALUES ($1, $2)",
                                &[&format!("User {}", i), &format!("user{}@example.com", i)],
                            )
                            .unwrap();
                    }
                    client
                })
                .bench_values(|mut client| {
                    let rows = client
                        .query(
                            r#"SELECT "bench_users"."id", "bench_users"."name", "bench_users"."email" FROM "bench_users""#,
                            &[],
                        )
                        .unwrap();

                    let results: Vec<_> = rows
                        .iter()
                        .map(|row| {
                            (
                                row.get::<_, i32>(0),
                                row.get::<_, String>(1),
                                row.get::<_, String>(2),
                            )
                        })
                        .collect();
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (mut db, users) = setup_postgres_drizzle();
                    db.insert(users).values(gen_users!(100)).execute().unwrap();
                    (db, users)
                })
                .bench_values(|(mut db, users)| {
                    let results: Vec<SelectUser> = db.select(()).from(users).all().unwrap();
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle_prepared(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (mut db, users) = setup_postgres_drizzle();
                    seed_users_fast(&mut db, 100);
                    let prepared = db.select(()).from(users).prepare().into_owned();
                    (db, prepared)
                })
                .bench_values(|(mut db, prepared)| {
                    let results: Vec<SelectUser> = prepared.all(db.mut_client(), []).unwrap();
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
                    let mut client = setup_postgres_connection();
                    for i in 0..100 {
                        client
                            .execute(
                                "INSERT INTO bench_users (name, email) VALUES ($1, $2)",
                                &[&format!("User {}", i), &format!("user{}@example.com", i)],
                            )
                            .unwrap();
                    }
                    client
                })
                .bench_values(|mut client| {
                    let rows = client
                        .query(
                            r#"SELECT "bench_users"."id", "bench_users"."name", "bench_users"."email" FROM "bench_users" WHERE "bench_users"."id" = $1"#,
                            &[&black_box(50i32)],
                        )
                        .unwrap();

                    let results: Vec<_> = rows
                        .iter()
                        .map(|row| {
                            (
                                row.get::<_, i32>(0),
                                row.get::<_, String>(1),
                                row.get::<_, String>(2),
                            )
                        })
                        .collect();
                    black_box(results);
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (mut db, users) = setup_postgres_drizzle();
                    db.insert(users).values(gen_users!(100)).execute().unwrap();
                    (db, users)
                })
                .bench_values(|(mut db, users)| {
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
                    let (mut db, users) = setup_postgres_drizzle();
                    seed_users_fast(&mut db, 100);
                    let prepared = db
                        .select(())
                        .from(users)
                        .r#where(eq(users.id, 50))
                        .prepare()
                        .into_owned();
                    (db, prepared)
                })
                .bench_values(|(mut db, prepared)| {
                    let results: Vec<SelectUser> = prepared.all(db.mut_client(), []).unwrap();
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
                .with_inputs(|| setup_postgres_connection())
                .bench_values(|mut client| {
                    client
                        .execute(
                            "INSERT INTO bench_users (name, email) VALUES ($1, $2)",
                            &[&black_box("user"), &black_box("user@example.com")],
                        )
                        .unwrap()
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| setup_postgres_drizzle())
                .bench_values(|(mut db, user)| {
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
                    let (mut db, user) = setup_postgres_drizzle();
                    let prepared = db
                        .insert(user)
                        .values([InsertUser::new("user", "user@example.com")])
                        .prepare()
                        .into_owned();
                    (db, prepared)
                })
                .bench_values(|(mut db, prepared)| {
                    prepared.execute(db.mut_client(), []).unwrap();
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
                    let mut client = setup_postgres_connection();
                    client
                        .execute(
                            "INSERT INTO bench_users (name, email) VALUES ('user', 'user@example.com')",
                            &[],
                        )
                        .unwrap();
                    client
                })
                .bench_values(|mut client| {
                    client
                        .execute(
                            r#"UPDATE "bench_users" SET "name" = $1 WHERE "bench_users"."id" = $2"#,
                            &[&black_box("updated"), &black_box(1i32)],
                        )
                        .unwrap()
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (mut db, users) = setup_postgres_drizzle();
                    db.insert(users)
                        .values([InsertUser::new("user", "user@example.com")])
                        .execute()
                        .unwrap();
                    (db, users)
                })
                .bench_values(|(mut db, users)| {
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
                    let (mut db, users) = setup_postgres_drizzle();
                    seed_one_user_fast(&mut db);
                    let prepared = db
                        .update(users)
                        .set(UpdateUser::default().with_name("updated"))
                        .r#where(eq(users.id, 1))
                        .prepare()
                        .into_owned();
                    (db, prepared)
                })
                .bench_values(|(mut db, prepared)| {
                    prepared.execute(db.mut_client(), []).unwrap();
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
                    let mut client = setup_postgres_connection();
                    for i in 0..10 {
                        client
                            .execute(
                                "INSERT INTO bench_users (name, email) VALUES ($1, $2)",
                                &[&format!("User {}", i), &format!("user{}@example.com", i)],
                            )
                            .unwrap();
                    }
                    client
                })
                .bench_values(|mut client| {
                    client
                        .execute(
                            r#"DELETE FROM "bench_users" WHERE "bench_users"."id" = $1"#,
                            &[&black_box(1i32)],
                        )
                        .unwrap()
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (mut db, users) = setup_postgres_drizzle();
                    db.insert(users).values(gen_users!(10)).execute().unwrap();
                    (db, users)
                })
                .bench_values(|(mut db, users)| {
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
                    let (mut db, users) = setup_postgres_drizzle();
                    seed_users_fast(&mut db, 10);
                    let prepared = db
                        .delete(users)
                        .r#where(eq(users.id, 1))
                        .prepare()
                        .into_owned();
                    (db, prepared)
                })
                .bench_values(|(mut db, prepared)| {
                    prepared.execute(db.mut_client(), []).unwrap();
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
                    let client = setup_postgres_connection();

                    let mut sql = String::from("INSERT INTO bench_users (name, email) VALUES ");
                    let mut params: Vec<String> = Vec::with_capacity(2000);

                    for i in 0..1000 {
                        if i > 0 {
                            sql.push_str(", ");
                        }
                        sql.push_str(&format!("(${}, ${})", i * 2 + 1, i * 2 + 2));
                        params.push(black_box(format!("User {}", i)));
                        params.push(black_box(format!("user{}@example.com", i)));
                    }

                    (client, sql, params)
                })
                .bench_values(|(mut client, sql, params)| {
                    let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = params
                        .iter()
                        .map(|p| p as &(dyn postgres::types::ToSql + Sync))
                        .collect();
                    client.execute(&sql, &param_refs[..]).unwrap();
                });
        }

        #[divan::bench]
        fn drizzle(bencher: Bencher) {
            bencher
                .with_inputs(|| {
                    let (db, users) = setup_postgres_drizzle();
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
                .bench_values(|(mut db, users, data)| {
                    db.insert(users).values(data).execute().unwrap();
                });
        }
    }

    #[divan::bench_group]
    mod complex {
        use super::*;

        mod join {
            use super::*;

            #[derive(Debug, PostgresFromRow)]
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
                        let mut client = setup_postgres_blog_connection();
                        for i in 0..10 {
                            client
                                .execute(
                                    "INSERT INTO bench_users (name, email) VALUES ($1, $2)",
                                    &[
                                        &format!("User {}", i),
                                        &format!("user{}@example.com", i),
                                    ],
                                )
                                .unwrap();
                        }
                        for i in 0..100 {
                            client
                                .execute(
                                    "INSERT INTO bench_posts (title, content, author_id) VALUES ($1, $2, $3)",
                                    &[
                                        &format!("Post {}", i),
                                        &format!("Content {}", i),
                                        &((i % 10) + 1),
                                    ],
                                )
                                .unwrap();
                        }
                        client
                    })
                    .bench_values(|mut client| {
                        let rows = client
                            .query(
                                r#"SELECT "bench_users"."name", "bench_posts"."title" FROM "bench_users"
                                   INNER JOIN "bench_posts" ON "bench_users"."id" = "bench_posts"."author_id""#,
                                &[],
                            )
                            .unwrap();

                        let results: Vec<_> = rows
                            .iter()
                            .map(|row| (row.get::<_, String>(0), row.get::<_, String>(1)))
                            .collect();
                        black_box(results);
                    });
            }

            #[divan::bench]
            fn drizzle(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let (mut db, users, posts) = setup_postgres_blog_drizzle();
                        db.insert(users).values(gen_users!(10)).execute().unwrap();
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
                    .bench_values(|(mut db, users, posts)| {
                        let results: Vec<JoinResult> = db
                            .select(JoinResult::default())
                            .from(users)
                            .join(posts, eq(users.id, posts.author_id))
                            .all()
                            .unwrap();
                        black_box(results);
                    });
            }
        }

        mod aggregate {
            use super::*;

            #[derive(Debug, PostgresFromRow)]
            #[allow(dead_code)]
            struct CountResult {
                count: i64,
            }

            #[divan::bench]
            fn raw(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let mut client = setup_postgres_connection();
                        for i in 0..100 {
                            client
                                .execute(
                                    "INSERT INTO bench_users (name, email) VALUES ($1, $2)",
                                    &[&format!("User {}", i), &format!("user{}@example.com", i)],
                                )
                                .unwrap();
                        }
                        client
                    })
                    .bench_values(|mut client| {
                        let row = client
                            .query_one(r#"SELECT COUNT(*) FROM "bench_users""#, &[])
                            .unwrap();
                        let count: i64 = row.get(0);
                        black_box(count);
                    });
            }

            #[divan::bench]
            fn drizzle(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let (mut db, users) = setup_postgres_drizzle();
                        db.insert(users).values(gen_users!(100)).execute().unwrap();
                        (db, users)
                    })
                    .bench_values(|(mut db, users)| {
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
                        let mut client = setup_postgres_connection();
                        for i in 0..100 {
                            client
                                .execute(
                                    "INSERT INTO bench_users (name, email) VALUES ($1, $2)",
                                    &[
                                        &format!("User {}", i),
                                        &format!("user{}@example.com", i),
                                    ],
                                )
                                .unwrap();
                        }
                        client
                    })
                    .bench_values(|mut client| {
                        let rows = client
                            .query(
                                r#"SELECT "bench_users"."id", "bench_users"."name", "bench_users"."email" FROM "bench_users"
                                   ORDER BY "bench_users"."name" ASC LIMIT 10 OFFSET 20"#,
                                &[],
                            )
                            .unwrap();

                        let results: Vec<_> = rows
                            .iter()
                            .map(|row| {
                                (
                                    row.get::<_, i32>(0),
                                    row.get::<_, String>(1),
                                    row.get::<_, String>(2),
                                )
                            })
                            .collect();
                        black_box(results);
                    });
            }

            #[divan::bench]
            fn drizzle(bencher: Bencher) {
                bencher
                    .with_inputs(|| {
                        let (mut db, users) = setup_postgres_drizzle();
                        db.insert(users).values(gen_users!(100)).execute().unwrap();
                        (db, users)
                    })
                    .bench_values(|(mut db, users)| {
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

fn main() {
    #[cfg(feature = "profiling")]
    let captured_frames: std::sync::Arc<
        std::sync::Mutex<Vec<std::sync::Arc<puffin::FrameData>>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    #[cfg(feature = "profiling")]
    let sink_id = {
        let captured_frames = std::sync::Arc::clone(&captured_frames);
        puffin::GlobalProfiler::lock().add_sink(Box::new(move |frame| {
            if let Ok(mut frames) = captured_frames.lock() {
                frames.push(frame);
            }
        }))
    };

    #[cfg(feature = "profiling")]
    {
        puffin::set_scopes_on(true);
        std::thread::spawn(|| {
            loop {
                puffin::GlobalProfiler::lock().new_frame();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
    }

    divan::main();

    #[cfg(feature = "profiling")]
    {
        use std::collections::HashMap;

        fn accumulate_scope_times(
            stream: &puffin::Stream,
            offset: u64,
            totals_ns: &mut HashMap<(puffin::ScopeId, String), i64>,
        ) {
            let Ok(reader) = puffin::Reader::with_offset(stream, offset) else {
                return;
            };
            for scope in reader.flatten() {
                *totals_ns
                    .entry((scope.id, scope.record.data.to_owned()))
                    .or_insert(0) += scope.record.duration_ns;
                accumulate_scope_times(stream, scope.child_begin_position, totals_ns);
            }
        }

        puffin::GlobalProfiler::lock().new_frame();
        let _ = puffin::GlobalProfiler::lock().remove_sink(sink_id);

        let frames = captured_frames
            .lock()
            .map(|f| f.clone())
            .unwrap_or_default();

        let mut frame_view = puffin::FrameView::default();
        let mut totals_ns: HashMap<(puffin::ScopeId, String), i64> = HashMap::new();

        for frame in &frames {
            frame_view.add_frame(frame.clone());
            if let Ok(unpacked) = frame.unpacked() {
                for stream_info in unpacked.thread_streams.values() {
                    accumulate_scope_times(&stream_info.stream, 0, &mut totals_ns);
                }
            }
        }

        let scopes = frame_view.scope_collection();
        let mut totals: Vec<(String, i64)> = totals_ns
            .into_iter()
            .map(|((id, data), total_ns)| {
                let base = scopes
                    .fetch_by_id(&id)
                    .map(|d| d.name().to_string())
                    .unwrap_or_else(|| format!("scope#{}", id.0));
                let name = if data.is_empty() {
                    base
                } else {
                    format!("{}::{}", base, data)
                };
                (name, total_ns)
            })
            .collect();

        totals.sort_by(|a, b| b.1.cmp(&a.1));

        println!("\n=== Puffin Scope Totals (postgres bench) ===");
        for (idx, (name, total_ns)) in totals.iter().take(20).enumerate() {
            println!(
                "{:>2}. {:<60} {:>10.3} ms",
                idx + 1,
                name,
                *total_ns as f64 / 1_000_000.0
            );
        }
    }
}
