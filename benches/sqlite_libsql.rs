// libsql benchmarks — separate binary to avoid sqlite3 C symbol conflicts
// between libsqlite3-sys (rusqlite) and libsql-ffi on Windows MSVC.
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

mod common;
use common::*;

#[allow(dead_code)]
async fn ls_raw() -> ::libsql::Connection {
    let db = ::libsql::Builder::new_local(":memory:")
        .build()
        .await
        .expect("db");
    let conn = db.connect().expect("connect");
    conn.execute(User::ddl_sql(), ()).await.expect("ddl");
    conn
}

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

criterion_group!(sqlite_libsql, bench_libsql);
criterion_main!(sqlite_libsql);
