use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use drizzle::core::expr::eq;
use drizzle::postgres::prelude::*;
use std::sync::Arc;

macro_rules! users {
    ($seed:expr, $trial:expr, $n:expr) => {
        (0..$n).map(|i| {
            InsertUser::new(
                format!("user-{}-{}-{}", $seed, $trial, i),
                format!("u{}-{}-{}@x.dev", $seed, $trial, i),
            )
        })
    };
}

macro_rules! posts {
    ($seed:expr, $trial:expr, $n:expr, $authors:expr) => {
        (0..$n).map(|i| {
            InsertPost::new(
                format!("post-{}-{}-{}", $seed, $trial, i),
                format!("body-{}-{}-{}", $seed, $trial, i),
                ((i % $authors) + 1) as i32,
            )
        })
    };
}

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
    body: String,
    author_id: i32,
}

#[derive(PostgresSchema)]
struct Schema {
    user: User,
    post: Post,
}

#[derive(Clone)]
struct AppState {
    db: Arc<tokio::sync::Mutex<drizzle::postgres::tokio::Drizzle<Schema>>>,
}

pub async fn serve(seed: u64, trial: u32) -> Result<ServerHandle, Fail> {
    let (conn, driver) = ::tokio_postgres::connect(&pg_url(), ::tokio_postgres::NoTls)
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("postgres connect failed: {err}")))?;
    tokio::spawn(async move {
        let _ = driver.await;
    });

    conn.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users;")
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("postgres drop failed: {err}")))?;
    let (db, schema) = drizzle::postgres::tokio::Drizzle::new(conn, Schema::new());
    db.create()
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("postgres create failed: {err}")))?;
    db.insert(schema.user)
        .values(users!(seed, trial, 256))
        .execute()
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("postgres seed users failed: {err}")))?;
    db.insert(schema.post)
        .values(posts!(seed, trial, 1024, 256))
        .execute()
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("postgres seed posts failed: {err}")))?;

    let router = Router::new()
        .route("/stats", get(stats))
        .route("/customers", get(customers))
        .route("/customer-by-id", get(customer_by_id))
        .route("/orders", get(orders))
        .route("/orders-with-details", get(orders_with_details))
        .with_state(AppState {
            db: Arc::new(tokio::sync::Mutex::new(db)),
        });
    spawn_server(router).await
}

#[debug_handler(state = AppState)]
async fn stats(_: State<AppState>) -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    Json(cpu_usage(&sys))
}

#[debug_handler(state = AppState)]
async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<UserRow>>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
    let rows: Vec<(i32, String, String)> = db
        .select((schema.user.id, schema.user.name, schema.user.email))
        .from(schema.user)
        .limit(20)
        .offset(params.page())
        .all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, name, email)| UserRow { id, name, email })
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<UserRow>>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
    let rows: Vec<(i32, String, String)> = db
        .select((schema.user.id, schema.user.name, schema.user.email))
        .from(schema.user)
        .r#where(eq(schema.user.id, params.user_id(256)))
        .all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, name, email)| UserRow { id, name, email })
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn orders(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<PostRow>>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
    let rows: Vec<(i32, String, i32)> = db
        .select((schema.post.id, schema.post.title, schema.post.author_id))
        .from(schema.post)
        .limit(20)
        .offset(params.page())
        .all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, title, author_id)| PostRow {
                id,
                title,
                author_id,
            })
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(_params): Query<QueryParams>,
) -> Result<Json<Vec<DetailRow>>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
    let rows: Vec<(String, String)> = db
        .select((schema.user.name, schema.post.title))
        .from(schema.user)
        .inner_join((schema.post, eq(schema.user.id, schema.post.author_id)))
        .all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|(name, title)| DetailRow { name, title })
            .collect(),
    ))
}
