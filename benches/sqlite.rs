use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle::core::expr::{count, eq};
#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle::sqlite::prelude::*;

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[SQLiteTable(name = "bench_users")]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
    email: String,
}

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[SQLiteTable(name = "bench_posts")]
struct Post {
    #[column(primary)]
    id: i32,
    title: String,
    body: String,
    author_id: i32,
}

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[derive(SQLiteSchema)]
struct Schema {
    user: User,
}

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[derive(SQLiteSchema)]
struct BlogSchema {
    user: User,
    post: Post,
}

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
macro_rules! users {
    ($n:expr) => {
        (0..$n).map(|i| InsertUser::new(format!("User {}", i), format!("user{}@x.dev", i)))
    };
}

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
macro_rules! posts {
    ($n:expr, $authors:expr) => {
        (0..$n).map(|i| {
            InsertPost::new(
                format!("Post {}", i),
                format!("Body {}", i),
                (i % $authors) + 1,
            )
        })
    };
}

#[cfg(feature = "rusqlite")]
fn rs_raw() -> ::rusqlite::Connection {
    let conn = ::rusqlite::Connection::open_in_memory().expect("open");
    conn.execute(User::ddl_sql(), []).expect("ddl");
    conn
}

#[cfg(feature = "rusqlite")]
fn rs_raw_blog() -> ::rusqlite::Connection {
    let conn = ::rusqlite::Connection::open_in_memory().expect("open");
    conn.execute(User::ddl_sql(), []).expect("ddl users");
    conn.execute(Post::ddl_sql(), []).expect("ddl posts");
    conn
}

#[cfg(feature = "rusqlite")]
fn rs_db() -> (drizzle::sqlite::rusqlite::Drizzle<Schema>, User) {
    let conn = ::rusqlite::Connection::open_in_memory().expect("open");
    let (db, Schema { user }) = drizzle::sqlite::rusqlite::Drizzle::new(conn, Schema::new());
    db.create().expect("create");
    (db, user)
}

#[cfg(feature = "rusqlite")]
fn rs_db_blog() -> (drizzle::sqlite::rusqlite::Drizzle<BlogSchema>, User, Post) {
    let conn = ::rusqlite::Connection::open_in_memory().expect("open");
    let (db, BlogSchema { user, post }) =
        drizzle::sqlite::rusqlite::Drizzle::new(conn, BlogSchema::new());
    db.create().expect("create");
    (db, user, post)
}

#[cfg(feature = "turso")]
async fn tu_raw() -> ::turso::Connection {
    let db = ::turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("db");
    let conn = db.connect().expect("connect");
    conn.execute(User::ddl_sql(), ()).await.expect("ddl");
    conn
}

#[cfg(feature = "turso")]
async fn tu_db() -> (drizzle::sqlite::turso::Drizzle<Schema>, User) {
    let db = ::turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("db");
    let conn = db.connect().expect("connect");
    let (db, Schema { user }) = drizzle::sqlite::turso::Drizzle::new(conn, Schema::new());
    db.execute(drizzle::core::SQL::raw(User::ddl_sql()))
        .await
        .expect("create");
    (db, user)
}

#[cfg(feature = "turso")]
async fn tu_db_blog() -> (drizzle::sqlite::turso::Drizzle<BlogSchema>, User, Post) {
    let db = ::turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("db");
    let conn = db.connect().expect("connect");
    let (db, BlogSchema { user, post }) =
        drizzle::sqlite::turso::Drizzle::new(conn, BlogSchema::new());
    db.execute(drizzle::core::SQL::raw(User::ddl_sql()))
        .await
        .expect("create users");
    db.execute(drizzle::core::SQL::raw(Post::ddl_sql()))
        .await
        .expect("create posts");
    (db, user, post)
}

#[cfg(feature = "libsql")]
async fn ls_raw() -> ::libsql::Connection {
    let db = ::libsql::Builder::new_local(":memory:")
        .build()
        .await
        .expect("db");
    let conn = db.connect().expect("connect");
    conn.execute(User::ddl_sql(), ()).await.expect("ddl");
    conn
}

#[cfg(feature = "libsql")]
async fn ls_db() -> (drizzle::sqlite::libsql::Drizzle<Schema>, User) {
    let db = ::libsql::Builder::new_local(":memory:")
        .build()
        .await
        .expect("db");
    let conn = db.connect().expect("connect");
    let (db, Schema { user }) = drizzle::sqlite::libsql::Drizzle::new(conn, Schema::new());
    db.execute(drizzle::core::SQL::raw(User::ddl_sql()))
        .await
        .expect("create");
    (db, user)
}

#[cfg(feature = "libsql")]
async fn ls_db_blog() -> (drizzle::sqlite::libsql::Drizzle<BlogSchema>, User, Post) {
    let db = ::libsql::Builder::new_local(":memory:")
        .build()
        .await
        .expect("db");
    let conn = db.connect().expect("connect");
    let (db, BlogSchema { user, post }) =
        drizzle::sqlite::libsql::Drizzle::new(conn, BlogSchema::new());
    db.execute(drizzle::core::SQL::raw(User::ddl_sql()))
        .await
        .expect("create users");
    db.execute(drizzle::core::SQL::raw(Post::ddl_sql()))
        .await
        .expect("create posts");
    (db, user, post)
}

#[cfg(feature = "rusqlite")]
fn bench_rusqlite(c: &mut Criterion) {
    use drizzle::sqlite::connection::SQLiteTransactionType;
    use drizzle_core::asc;

    let mut read = c.benchmark_group("rusqlite/read");
    read.bench_function("select_raw", |b| {
        b.iter_batched(
            || {
                let conn = rs_raw();
                for i in 0..100 {
                    conn.execute(
                        "INSERT INTO bench_users (name, email) VALUES (?1, ?2)",
                        [format!("User {}", i), format!("user{}@x.dev", i)],
                    )
                    .expect("seed");
                }
                conn
            },
            |conn| {
                let mut stmt = conn
                    .prepare("SELECT id, name, email FROM bench_users")
                    .expect("prepare");
                let rows = stmt
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, i32>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                        ))
                    })
                    .expect("query");
                black_box(rows.collect::<Vec<_>>());
            },
            criterion::BatchSize::SmallInput,
        );
    });
    read.bench_function("select_drizzle", |b| {
        b.iter_batched(
            || {
                let (db, user) = rs_db();
                db.insert(user).values(users!(100)).execute().expect("seed");
                (db, user)
            },
            |(db, user)| {
                let out: Vec<(i32, String, String)> = db
                    .select((user.id, user.name, user.email))
                    .from(user)
                    .all()
                    .expect("select");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    read.finish();

    let mut write = c.benchmark_group("rusqlite/write");
    write.bench_function("insert", |b| {
        b.iter_batched(
            rs_db,
            |(db, user)| {
                db.insert(user)
                    .values([InsertUser::new("one", "one@x.dev")])
                    .execute()
                    .expect("insert");
            },
            criterion::BatchSize::SmallInput,
        );
    });
    write.bench_function("update", |b| {
        b.iter_batched(
            || {
                let (db, user) = rs_db();
                db.insert(user)
                    .values([InsertUser::new("a", "a@x.dev").with_id(1)])
                    .execute()
                    .expect("seed");
                (db, user)
            },
            |(db, user)| {
                db.update(user)
                    .set(UpdateUser::default().with_name("b"))
                    .r#where(eq(user.id, 1))
                    .execute()
                    .expect("update");
            },
            criterion::BatchSize::SmallInput,
        );
    });
    write.bench_function("delete", |b| {
        b.iter_batched(
            || {
                let (db, user) = rs_db();
                db.insert(user)
                    .values([InsertUser::new("a", "a@x.dev").with_id(1)])
                    .execute()
                    .expect("seed");
                (db, user)
            },
            |(db, user)| {
                db.delete(user)
                    .r#where(eq(user.id, 1))
                    .execute()
                    .expect("delete");
            },
            criterion::BatchSize::SmallInput,
        );
    });
    write.finish();

    let mut bulk = c.benchmark_group("rusqlite/bulk");
    bulk.bench_function("drizzle", |b| {
        b.iter_batched(
            rs_db,
            |(db, user)| {
                db.insert(user)
                    .values(users!(1_000))
                    .execute()
                    .expect("bulk");
            },
            criterion::BatchSize::SmallInput,
        );
    });
    bulk.finish();

    let mut query = c.benchmark_group("rusqlite/query");
    query.bench_function("join", |b| {
        b.iter_batched(
            || {
                let (db, user, post) = rs_db_blog();
                db.insert(user)
                    .values(users!(10))
                    .execute()
                    .expect("seed users");
                db.insert(post)
                    .values(posts!(100, 10))
                    .execute()
                    .expect("seed posts");
                (db, user, post)
            },
            |(db, user, post)| {
                let out: Vec<(String, String)> = db
                    .select((user.name, post.title))
                    .from(user)
                    .inner_join((post, eq(user.id, post.author_id)))
                    .all()
                    .expect("join");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    query.bench_function("agg", |b| {
        b.iter_batched(
            || {
                let (db, user) = rs_db();
                db.insert(user).values(users!(100)).execute().expect("seed");
                (db, user)
            },
            |(db, user)| {
                let out: Vec<(i32,)> = db.select((count(user.id),)).from(user).all().expect("agg");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    query.bench_function("page", |b| {
        b.iter_batched(
            || {
                let (db, user) = rs_db();
                db.insert(user).values(users!(100)).execute().expect("seed");
                (db, user)
            },
            |(db, user)| {
                let out: Vec<(i32, String, String)> = db
                    .select((user.id, user.name, user.email))
                    .from(user)
                    .order_by([asc(user.name)])
                    .limit(10)
                    .offset(20)
                    .all()
                    .expect("page");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    query.finish();

    let mut extra = c.benchmark_group("rusqlite/extra");
    extra.bench_function("tx", |b| {
        b.iter_batched(
            rs_db,
            |(mut db, user)| {
                db.transaction(SQLiteTransactionType::Immediate, |tx| {
                    tx.insert(user).values(users!(25)).execute()?;
                    Ok::<(), drizzle::error::DrizzleError>(())
                })
                .expect("tx");
            },
            criterion::BatchSize::SmallInput,
        );
    });
    extra.bench_function("upsert", |b| {
        b.iter_batched(
            rs_db,
            |(db, user)| {
                db.insert(user)
                    .values([InsertUser::new("up", "up@x.dev").with_id(1)])
                    .on_conflict(user.id)
                    .do_update(UpdateUser::default().with_name("up").with_email("up@x.dev"))
                    .execute()
                    .expect("upsert");
            },
            criterion::BatchSize::SmallInput,
        );
    });
    extra.bench_function("ret", |b| {
        b.iter_batched(
            rs_db,
            |(db, user)| {
                let out: Vec<(i32, String, String)> = db
                    .insert(user)
                    .values([InsertUser::new("ret", "ret@x.dev")])
                    .returning((user.id, user.name, user.email))
                    .all()
                    .expect("ret");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    extra.bench_function("sql", |b| {
        b.iter_batched(
            rs_db,
            |(db, user)| {
                let sql = db
                    .select((user.id,))
                    .from(user)
                    .r#where(eq(user.id, 42))
                    .to_sql();
                black_box(sql.sql());
                black_box(sql.params().count());
            },
            criterion::BatchSize::SmallInput,
        );
    });
    extra.finish();

    let mut scale = c.benchmark_group("rusqlite/scale");
    for n in [10, 100, 1_000] {
        scale.throughput(Throughput::Elements(n as u64));
        scale.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let (db, user) = rs_db();
                    db.insert(user).values(users!(n)).execute().expect("seed");
                    (db, user)
                },
                |(db, user)| {
                    let out: Vec<(i32, String, String)> = db
                        .select((user.id, user.name, user.email))
                        .from(user)
                        .r#where(eq(user.id, n / 2))
                        .all()
                        .expect("where");
                    black_box(out);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    scale.finish();

    let _ = rs_raw_blog;
}

#[cfg(feature = "turso")]
fn bench_turso(c: &mut Criterion) {
    use drizzle::sqlite::connection::SQLiteTransactionType;
    use drizzle_core::asc;

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let mut read = c.benchmark_group("turso/read");
    read.bench_function("select", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tu_db().await;
                db.insert(user)
                    .values(users!(100))
                    .execute()
                    .await
                    .expect("seed");
                let out: Vec<(i32, String, String)> = db
                    .select((user.id, user.name, user.email))
                    .from(user)
                    .all()
                    .await
                    .expect("select");
                black_box(out);
            })
        })
    });
    read.finish();

    let mut write = c.benchmark_group("turso/write");
    write.bench_function("insert", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tu_db().await;
                db.insert(user)
                    .values([InsertUser::new("one", "one@x.dev")])
                    .execute()
                    .await
                    .expect("insert");
            })
        })
    });
    write.bench_function("tx", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (mut db, user) = tu_db().await;
                db.transaction(SQLiteTransactionType::Immediate, async |tx| {
                    tx.insert(user).values(users!(25)).execute().await?;
                    Ok::<(), drizzle::error::DrizzleError>(())
                })
                .await
                .expect("tx");
            })
        })
    });
    write.bench_function("upsert", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tu_db().await;
                db.insert(user)
                    .values([InsertUser::new("up", "up@x.dev").with_id(1)])
                    .on_conflict(user.id)
                    .do_update(UpdateUser::default().with_name("up").with_email("up@x.dev"))
                    .execute()
                    .await
                    .expect("upsert");
            })
        })
    });
    // FIXME: turso RETURNING is broken upstream — re-enable once fixed
    // write.bench_function("ret", |b| {
    //     b.iter(|| {
    //         rt.block_on(async {
    //             let (db, user) = tu_db().await;
    //             let out: Vec<(i32, String, String)> = db
    //                 .insert(user)
    //                 .values([InsertUser::new("ret", "ret@x.dev")])
    //                 .returning((user.id, user.name, user.email))
    //                 .all()
    //                 .await
    //                 .expect("ret");
    //             black_box(out);
    //         })
    //     })
    // });
    write.finish();

    let mut query = c.benchmark_group("turso/query");
    query.bench_function("join", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user, post) = tu_db_blog().await;
                db.insert(user)
                    .values(users!(10))
                    .execute()
                    .await
                    .expect("seed users");
                db.insert(post)
                    .values(posts!(100, 10))
                    .execute()
                    .await
                    .expect("seed posts");
                let out: Vec<(String, String)> = db
                    .select((user.name, post.title))
                    .from(user)
                    .inner_join((post, eq(user.id, post.author_id)))
                    .all()
                    .await
                    .expect("join");
                black_box(out);
            })
        })
    });
    query.bench_function("agg", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tu_db().await;
                db.insert(user)
                    .values(users!(100))
                    .execute()
                    .await
                    .expect("seed");
                let out: Vec<(i32,)> = db
                    .select((count(user.id),))
                    .from(user)
                    .all()
                    .await
                    .expect("agg");
                black_box(out);
            })
        })
    });
    query.bench_function("page", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tu_db().await;
                db.insert(user)
                    .values(users!(100))
                    .execute()
                    .await
                    .expect("seed");
                let out: Vec<(i32, String, String)> = db
                    .select((user.id, user.name, user.email))
                    .from(user)
                    .order_by([asc(user.name)])
                    .limit(10)
                    .offset(20)
                    .all()
                    .await
                    .expect("page");
                black_box(out);
            })
        })
    });
    query.finish();

    let mut extra = c.benchmark_group("turso/extra");
    extra.bench_function("sql", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tu_db().await;
                let sql = db
                    .select((user.id,))
                    .from(user)
                    .r#where(eq(user.id, 42))
                    .to_sql();
                black_box(sql.sql());
                black_box(sql.params().count());
            })
        })
    });
    extra.finish();

    let mut scale = c.benchmark_group("turso/scale");
    for n in [10, 100, 1_000] {
        scale.throughput(Throughput::Elements(n as u64));
        scale.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                rt.block_on(async {
                    let (db, user) = tu_db().await;
                    db.insert(user)
                        .values(users!(n))
                        .execute()
                        .await
                        .expect("seed");
                    let out: Vec<(i32, String, String)> = db
                        .select((user.id, user.name, user.email))
                        .from(user)
                        .r#where(eq(user.id, n / 2))
                        .all()
                        .await
                        .expect("where");
                    black_box(out);
                })
            });
        });
    }
    scale.finish();

    let _ = tu_raw;
}

#[cfg(feature = "libsql")]
fn bench_libsql(c: &mut Criterion) {
    use drizzle::sqlite::connection::SQLiteTransactionType;
    use drizzle_core::asc;

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let mut read = c.benchmark_group("libsql/read");
    read.bench_function("select", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = ls_db().await;
                db.insert(user)
                    .values(users!(100))
                    .execute()
                    .await
                    .expect("seed");
                let out: Vec<(i32, String, String)> = db
                    .select((user.id, user.name, user.email))
                    .from(user)
                    .all()
                    .await
                    .expect("select");
                black_box(out);
            })
        })
    });
    read.finish();

    let mut write = c.benchmark_group("libsql/write");
    write.bench_function("insert", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = ls_db().await;
                db.insert(user)
                    .values([InsertUser::new("one", "one@x.dev")])
                    .execute()
                    .await
                    .expect("insert");
            })
        })
    });
    write.bench_function("tx", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = ls_db().await;
                db.transaction(SQLiteTransactionType::Immediate, async |tx| {
                    tx.insert(user).values(users!(25)).execute().await?;
                    Ok::<(), drizzle::error::DrizzleError>(())
                })
                .await
                .expect("tx");
            })
        })
    });
    write.bench_function("upsert", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = ls_db().await;
                db.insert(user)
                    .values([InsertUser::new("up", "up@x.dev").with_id(1)])
                    .on_conflict(user.id)
                    .do_update(UpdateUser::default().with_name("up").with_email("up@x.dev"))
                    .execute()
                    .await
                    .expect("upsert");
            })
        })
    });
    write.bench_function("ret", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = ls_db().await;
                let out: Vec<(i32, String, String)> = db
                    .insert(user)
                    .values([InsertUser::new("ret", "ret@x.dev")])
                    .returning((user.id, user.name, user.email))
                    .all()
                    .await
                    .expect("ret");
                black_box(out);
            })
        })
    });
    write.finish();

    let mut query = c.benchmark_group("libsql/query");
    query.bench_function("join", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user, post) = ls_db_blog().await;
                db.insert(user)
                    .values(users!(10))
                    .execute()
                    .await
                    .expect("seed users");
                db.insert(post)
                    .values(posts!(100, 10))
                    .execute()
                    .await
                    .expect("seed posts");
                let out: Vec<(String, String)> = db
                    .select((user.name, post.title))
                    .from(user)
                    .inner_join((post, eq(user.id, post.author_id)))
                    .all()
                    .await
                    .expect("join");
                black_box(out);
            })
        })
    });
    query.bench_function("agg", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = ls_db().await;
                db.insert(user)
                    .values(users!(100))
                    .execute()
                    .await
                    .expect("seed");
                let out: Vec<(i32,)> = db
                    .select((count(user.id),))
                    .from(user)
                    .all()
                    .await
                    .expect("agg");
                black_box(out);
            })
        })
    });
    query.bench_function("page", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = ls_db().await;
                db.insert(user)
                    .values(users!(100))
                    .execute()
                    .await
                    .expect("seed");
                let out: Vec<(i32, String, String)> = db
                    .select((user.id, user.name, user.email))
                    .from(user)
                    .order_by([asc(user.name)])
                    .limit(10)
                    .offset(20)
                    .all()
                    .await
                    .expect("page");
                black_box(out);
            })
        })
    });
    query.finish();

    let mut extra = c.benchmark_group("libsql/extra");
    extra.bench_function("sql", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = ls_db().await;
                let sql = db
                    .select((user.id,))
                    .from(user)
                    .r#where(eq(user.id, 42))
                    .to_sql();
                black_box(sql.sql());
                black_box(sql.params().count());
            })
        })
    });
    extra.finish();

    let mut scale = c.benchmark_group("libsql/scale");
    for n in [10, 100, 1_000] {
        scale.throughput(Throughput::Elements(n as u64));
        scale.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                rt.block_on(async {
                    let (db, user) = ls_db().await;
                    db.insert(user)
                        .values(users!(n))
                        .execute()
                        .await
                        .expect("seed");
                    let out: Vec<(i32, String, String)> = db
                        .select((user.id, user.name, user.email))
                        .from(user)
                        .r#where(eq(user.id, n / 2))
                        .all()
                        .await
                        .expect("where");
                    black_box(out);
                })
            });
        });
    }
    scale.finish();

    let _ = ls_raw;
}

#[cfg(all(feature = "rusqlite", feature = "turso"))]
fn bench_mvcc(c: &mut Criterion) {
    use drizzle::sqlite::connection::SQLiteTransactionType;
    use std::sync::mpsc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn path(prefix: &str) -> String {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir()
            .join(format!(
                "drizzle-{}-{}-{}.db",
                prefix,
                std::process::id(),
                n
            ))
            .to_string_lossy()
            .into_owned()
    }

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let mut g = c.benchmark_group("mvcc/readwrite");

    g.bench_function("rusqlite", |b| {
        b.iter_batched(
            || {
                let p = path("rs");
                let conn = ::rusqlite::Connection::open(&p).expect("open");
                let _ = conn.execute("PRAGMA journal_mode=WAL", []);
                conn.execute(User::ddl_sql(), []).expect("ddl");
                conn.execute(
                    "INSERT INTO bench_users (id, name, email) VALUES (1, 'seed', 'seed@x.dev')",
                    [],
                )
                .expect("seed");
                p
            },
            |p| {
                let (tx, rx) = mpsc::channel();
                let p2 = p.clone();
                let writer = std::thread::spawn(move || {
                    let conn = ::rusqlite::Connection::open(&p2).expect("open");
                    conn.execute("BEGIN IMMEDIATE", []).expect("begin");
                    conn.execute("UPDATE bench_users SET name = 'writer' WHERE id = 1", [])
                        .expect("update");
                    let _ = tx.send(());
                    std::thread::sleep(Duration::from_millis(2));
                    conn.execute("COMMIT", []).expect("commit");
                });

                let _ = rx.recv();
                let conn = ::rusqlite::Connection::open(&p).expect("open");
                let (db, Schema { user }) =
                    drizzle::sqlite::rusqlite::Drizzle::new(conn, Schema::new());
                let out: Vec<(i32, String, String)> = db
                    .select((user.id, user.name, user.email))
                    .from(user)
                    .r#where(eq(user.id, 1))
                    .all()
                    .expect("read");
                black_box(out);
                let _ = writer.join();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    g.bench_function("turso", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let p = path("tu");
                    let db = ::turso::Builder::new_local(&p).build().await.expect("db");
                    let conn = db.connect().expect("connect");
                    conn.execute(User::ddl_sql(), ()).await.expect("ddl");
                    conn.execute(
                        "INSERT INTO bench_users (id, name, email) VALUES (1, 'seed', 'seed@x.dev')",
                        (),
                    )
                    .await
                    .expect("seed");
                    p
                })
            },
            |p| {
                rt.block_on(async {
                    let db = ::turso::Builder::new_local(&p).build().await.expect("db");
                    let wc = db.connect().expect("wc");
                    let rc = db.connect().expect("rc");

                    let (mut wdb, Schema { user: wu }) =
                        drizzle::sqlite::turso::Drizzle::new(wc, Schema::new());
                    let (rdb, Schema { user: ru }) =
                        drizzle::sqlite::turso::Drizzle::new(rc, Schema::new());

                    let w = async {
                        wdb.transaction(SQLiteTransactionType::Immediate, async |tx| {
                            tx.update(wu)
                                .set(UpdateUser::default().with_name("writer"))
                                .r#where(eq(wu.id, 1))
                                .execute()
                                .await?;
                            tokio::time::sleep(Duration::from_millis(2)).await;
                            Ok::<(), drizzle::error::DrizzleError>(())
                        })
                        .await
                        .expect("tx");
                    };

                    let r = async {
                        tokio::time::sleep(Duration::from_micros(200)).await;
                        let out: Vec<(i32, String, String)> = rdb
                            .select((ru.id, ru.name, ru.email))
                            .from(ru)
                            .r#where(eq(ru.id, 1))
                            .all()
                            .await
                            .expect("read");
                        black_box(out);
                    };

                    tokio::join!(w, r);
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    g.finish();
}

fn bench_sqlite(c: &mut Criterion) {
    // libsql must run before rusqlite: libsql asserts a specific sqlite
    // threading mode (21) at init, while rusqlite is permissive. If rusqlite
    // initializes sqlite first, it sets a different mode and libsql panics.
    #[cfg(feature = "libsql")]
    bench_libsql(c);

    #[cfg(feature = "rusqlite")]
    bench_rusqlite(c);

    #[cfg(feature = "turso")]
    bench_turso(c);

    #[cfg(all(feature = "rusqlite", feature = "turso"))]
    bench_mvcc(c);
}

criterion_group!(sqlite, bench_sqlite);
criterion_main!(sqlite);
