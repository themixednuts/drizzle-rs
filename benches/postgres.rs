#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use criterion::{BenchmarkId, Throughput};
use criterion::{Criterion, criterion_group, criterion_main};
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use std::hint::black_box;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use drizzle::core::expr::{count, eq};
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use drizzle::postgres::prelude::*;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
#[PostgresTable(name = "bench_users")]
struct User {
    #[column(serial, primary)]
    id: i32,
    name: String,
    email: String,
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
#[PostgresTable(name = "bench_posts")]
struct Post {
    #[column(serial, primary)]
    id: i32,
    title: String,
    body: String,
    author_id: i32,
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
#[derive(PostgresSchema)]
struct Schema {
    user: User,
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
#[derive(PostgresSchema)]
struct BlogSchema {
    user: User,
    post: Post,
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
macro_rules! users {
    ($n:expr) => {
        (0..$n).map(|i| InsertUser::new(format!("User {}", i), format!("user{}@x.dev", i)))
    };
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
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

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn url() -> String {
    let default = "host=localhost user=postgres password=postgres dbname=drizzle_test";
    let raw = std::env::var("DATABASE_URL").unwrap_or_default();
    if raw.trim().is_empty() {
        return default.to_string();
    }

    let mut parts = Vec::new();
    let mut saw_port = false;
    let mut valid_port = false;

    for part in raw.split_whitespace() {
        if let Some(port) = part.strip_prefix("port=") {
            saw_port = true;
            if port.parse::<u16>().is_ok() {
                valid_port = true;
                parts.push(part.to_string());
            }
        } else {
            parts.push(part.to_string());
        }
    }

    if saw_port && !valid_port {
        parts.push("port=5432".to_string());
    }

    if parts.is_empty() {
        default.to_string()
    } else {
        parts.join(" ")
    }
}

#[cfg(feature = "postgres-sync")]
fn ps_raw() -> ::postgres::Client {
    let mut c = ::postgres::Client::connect(&url(), ::postgres::NoTls).expect("connect");
    c.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .expect("drop");
    c.batch_execute(User::ddl_sql()).expect("ddl");
    c
}

#[cfg(feature = "postgres-sync")]
fn ps_raw_blog() -> ::postgres::Client {
    let mut c = ::postgres::Client::connect(&url(), ::postgres::NoTls).expect("connect");
    c.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .expect("drop");
    c.batch_execute(User::ddl_sql()).expect("ddl users");
    c.batch_execute(Post::ddl_sql()).expect("ddl posts");
    c
}

#[cfg(feature = "postgres-sync")]
fn ps_db() -> (drizzle::postgres::sync::Drizzle<Schema>, User) {
    let mut c = ::postgres::Client::connect(&url(), ::postgres::NoTls).expect("connect");
    c.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .expect("drop");
    let (mut db, Schema { user }) = drizzle::postgres::sync::Drizzle::new(c, Schema::new());
    db.create().expect("create");
    (db, user)
}

#[cfg(feature = "postgres-sync")]
fn ps_db_blog() -> (drizzle::postgres::sync::Drizzle<BlogSchema>, User, Post) {
    let mut c = ::postgres::Client::connect(&url(), ::postgres::NoTls).expect("connect");
    c.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .expect("drop");
    let (mut db, BlogSchema { user, post }) =
        drizzle::postgres::sync::Drizzle::new(c, BlogSchema::new());
    db.create().expect("create");
    (db, user, post)
}

#[cfg(feature = "tokio-postgres")]
async fn tp_raw() -> ::tokio_postgres::Client {
    let (c, conn) = ::tokio_postgres::connect(&url(), ::tokio_postgres::NoTls)
        .await
        .expect("connect");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    c.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .await
        .expect("drop");
    c.batch_execute(User::ddl_sql()).await.expect("ddl");
    c
}

#[cfg(feature = "tokio-postgres")]
async fn tp_db() -> (drizzle::postgres::tokio::Drizzle<Schema>, User) {
    let (c, conn) = ::tokio_postgres::connect(&url(), ::tokio_postgres::NoTls)
        .await
        .expect("connect");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    c.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .await
        .expect("drop");
    let (db, Schema { user }) = drizzle::postgres::tokio::Drizzle::new(c, Schema::new());
    db.create().await.expect("create");
    (db, user)
}

#[cfg(feature = "tokio-postgres")]
async fn tp_db_blog() -> (drizzle::postgres::tokio::Drizzle<BlogSchema>, User, Post) {
    let (c, conn) = ::tokio_postgres::connect(&url(), ::tokio_postgres::NoTls)
        .await
        .expect("connect");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    c.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users")
        .await
        .expect("drop");
    let (db, BlogSchema { user, post }) =
        drizzle::postgres::tokio::Drizzle::new(c, BlogSchema::new());
    db.create().await.expect("create");
    (db, user, post)
}

#[cfg(feature = "postgres-sync")]
fn bench_sync(c: &mut Criterion) {
    use drizzle_core::asc;
    use drizzle_postgres::common::PostgresTransactionType;

    let mut read = c.benchmark_group("sync/read");
    read.bench_function("select", |b| {
        b.iter_batched(
            || {
                let mut c = ps_raw();
                for i in 0..100 {
                    c.execute(
                        "INSERT INTO bench_users (name, email) VALUES ($1, $2)",
                        &[&format!("User {}", i), &format!("user{}@x.dev", i)],
                    )
                    .expect("seed");
                }
                c
            },
            |mut c| {
                let rows = c
                    .query("SELECT id, name, email FROM bench_users", &[])
                    .expect("query");
                let out: Vec<_> = rows
                    .iter()
                    .map(|r| {
                        (
                            r.get::<_, i32>(0),
                            r.get::<_, String>(1),
                            r.get::<_, String>(2),
                        )
                    })
                    .collect();
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    read.bench_function("where", |b| {
        b.iter_batched(
            || {
                let (mut db, user) = ps_db();
                db.insert(user).values(users!(100)).execute().expect("seed");
                (db, user)
            },
            |(mut db, user)| {
                let out: Vec<(i32, String, String)> = db
                    .select((user.id, user.name, user.email))
                    .from(user)
                    .r#where(eq(user.id, 50))
                    .all()
                    .expect("where");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    read.finish();

    let mut write = c.benchmark_group("sync/write");
    write.bench_function("insert", |b| {
        b.iter_batched(
            ps_db,
            |(mut db, user)| {
                db.insert(user)
                    .values([InsertUser::new("one", "one@x.dev")])
                    .execute()
                    .expect("insert");
            },
            criterion::BatchSize::SmallInput,
        )
    });
    write.bench_function("update", |b| {
        b.iter_batched(
            || {
                let (mut db, user) = ps_db();
                db.insert(user)
                    .values([InsertUser::new("a", "a@x.dev").with_id(1)])
                    .execute()
                    .expect("seed");
                (db, user)
            },
            |(mut db, user)| {
                db.update(user)
                    .set(UpdateUser::default().with_name("b"))
                    .r#where(eq(user.id, 1))
                    .execute()
                    .expect("update");
            },
            criterion::BatchSize::SmallInput,
        )
    });
    write.bench_function("delete", |b| {
        b.iter_batched(
            || {
                let (mut db, user) = ps_db();
                db.insert(user)
                    .values([InsertUser::new("a", "a@x.dev").with_id(1)])
                    .execute()
                    .expect("seed");
                (db, user)
            },
            |(mut db, user)| {
                db.delete(user)
                    .r#where(eq(user.id, 1))
                    .execute()
                    .expect("delete");
            },
            criterion::BatchSize::SmallInput,
        )
    });
    write.finish();

    let mut query = c.benchmark_group("sync/query");
    query.bench_function("join", |b| {
        b.iter_batched(
            || {
                let (mut db, user, post) = ps_db_blog();
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
            |(mut db, user, post)| {
                let out: Vec<(String, String)> = db
                    .select((user.name, post.title))
                    .from(user)
                    .inner_join((post, eq(user.id, post.author_id)))
                    .all()
                    .expect("join");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        )
    });
    query.bench_function("agg", |b| {
        b.iter_batched(
            || {
                let (mut db, user) = ps_db();
                db.insert(user).values(users!(100)).execute().expect("seed");
                (db, user)
            },
            |(mut db, user)| {
                let out: Vec<(i64,)> = db.select((count(user.id),)).from(user).all().expect("agg");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        )
    });
    query.bench_function("page", |b| {
        b.iter_batched(
            || {
                let (mut db, user) = ps_db();
                db.insert(user).values(users!(100)).execute().expect("seed");
                (db, user)
            },
            |(mut db, user)| {
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
        )
    });
    query.finish();

    let mut extra = c.benchmark_group("sync/extra");
    extra.bench_function("tx", |b| {
        b.iter_batched(
            ps_db,
            |(mut db, user)| {
                db.transaction(PostgresTransactionType::default(), |tx| {
                    tx.insert(user).values(users!(25)).execute()?;
                    Ok::<(), drizzle::error::DrizzleError>(())
                })
                .expect("tx");
            },
            criterion::BatchSize::SmallInput,
        )
    });
    extra.bench_function("upsert", |b| {
        b.iter_batched(
            ps_db,
            |(mut db, user)| {
                db.insert(user)
                    .values([InsertUser::new("up", "up@x.dev").with_id(1)])
                    .on_conflict(user.id)
                    .do_update(UpdateUser::default().with_name("up").with_email("up@x.dev"))
                    .execute()
                    .expect("upsert");
            },
            criterion::BatchSize::SmallInput,
        )
    });
    extra.bench_function("ret", |b| {
        b.iter_batched(
            ps_db,
            |(mut db, user)| {
                let out: Vec<(i32, String, String)> = db
                    .insert(user)
                    .values([InsertUser::new("ret", "ret@x.dev")])
                    .returning((user.id, user.name, user.email))
                    .all()
                    .expect("ret");
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        )
    });
    extra.bench_function("sql", |b| {
        b.iter_batched(
            ps_db,
            |(mut db, user)| {
                let sql = db
                    .select((user.id,))
                    .from(user)
                    .r#where(eq(user.id, 42))
                    .to_sql();
                black_box(sql.sql());
                black_box(sql.params().count());
            },
            criterion::BatchSize::SmallInput,
        )
    });
    extra.finish();

    let mut scale = c.benchmark_group("sync/scale");
    for n in [10, 100, 1_000] {
        scale.throughput(Throughput::Elements(n as u64));
        scale.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let (mut db, user) = ps_db();
                    db.insert(user).values(users!(n)).execute().expect("seed");
                    (db, user)
                },
                |(mut db, user)| {
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

    let _ = ps_raw_blog;
}

#[cfg(feature = "tokio-postgres")]
fn bench_tokio(c: &mut Criterion) {
    use drizzle_core::asc;
    use drizzle_postgres::common::PostgresTransactionType;

    let rt = tokio::runtime::Runtime::new().expect("runtime");

    let mut read = c.benchmark_group("tokio/read");
    read.bench_function("select", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tp_db().await;
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

    let mut write = c.benchmark_group("tokio/write");
    write.bench_function("insert", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tp_db().await;
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
                let (mut db, user) = tp_db().await;
                db.transaction(PostgresTransactionType::default(), async |tx| {
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
                let (db, user) = tp_db().await;
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
                let (db, user) = tp_db().await;
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

    let mut query = c.benchmark_group("tokio/query");
    query.bench_function("join", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user, post) = tp_db_blog().await;
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
                    .join((post, eq(user.id, post.author_id)))
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
                let (db, user) = tp_db().await;
                db.insert(user)
                    .values(users!(100))
                    .execute()
                    .await
                    .expect("seed");
                let out: Vec<(i64,)> = db
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
                let (db, user) = tp_db().await;
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

    let mut extra = c.benchmark_group("tokio/extra");
    extra.bench_function("sql", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (db, user) = tp_db().await;
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

    let mut scale = c.benchmark_group("tokio/scale");
    for n in [10, 100, 1_000] {
        scale.throughput(Throughput::Elements(n as u64));
        scale.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                rt.block_on(async {
                    let (db, user) = tp_db().await;
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

    let _ = tp_raw;
}

fn bench_postgres(c: &mut Criterion) {
    #[cfg(feature = "postgres-sync")]
    bench_sync(c);

    #[cfg(feature = "tokio-postgres")]
    bench_tokio(c);

    #[cfg(not(any(feature = "postgres-sync", feature = "tokio-postgres")))]
    let _ = c;
}

criterion_group!(postgres, bench_postgres);
criterion_main!(postgres);
