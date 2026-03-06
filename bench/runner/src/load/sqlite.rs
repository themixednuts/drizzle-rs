use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
use std::sync::{Arc, Mutex};

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

#[SQLiteTable(name = "bench_users")]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
    email: String,
}

#[SQLiteTable(name = "bench_posts")]
struct Post {
    #[column(primary)]
    id: i32,
    title: String,
    body: String,
    author_id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
    post: Post,
}

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<drizzle::sqlite::rusqlite::Drizzle<Schema>>>,
}

pub async fn serve(seed: u64, trial: u32) -> Result<ServerHandle, Fail> {
    let db = tokio::task::spawn_blocking(move || -> Result<_, Fail> {
        let conn = ::rusqlite::Connection::open_in_memory()
            .map_err(|err| Fail::new(Code::RunFail, format!("sqlite open failed: {err}")))?;
        let (db, schema) = drizzle::sqlite::rusqlite::Drizzle::new(conn, Schema::new());
        db.create()
            .map_err(|err| Fail::new(Code::RunFail, format!("sqlite create failed: {err}")))?;
        db.insert(schema.user)
            .values(users!(seed, trial, 256))
            .execute()
            .map_err(|err| Fail::new(Code::RunFail, format!("sqlite seed users failed: {err}")))?;
        db.insert(schema.post)
            .values(posts!(seed, trial, 1024, 256))
            .execute()
            .map_err(|err| Fail::new(Code::RunFail, format!("sqlite seed posts failed: {err}")))?;
        Ok(db)
    })
    .await
    .map_err(|err| Fail::new(Code::RunFail, format!("sqlite setup panicked: {err}")))??;

    let router = Router::new()
        .route("/stats", get(stats))
        .route("/customers", get(customers))
        .route("/customer-by-id", get(customer_by_id))
        .route("/orders", get(orders))
        .route("/orders-with-details", get(orders_with_details))
        .with_state(AppState {
            db: Arc::new(Mutex::new(db)),
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
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows: Vec<(i32, String, String)> = db
        .select((schema.user.id, schema.user.name, schema.user.email))
        .from(schema.user)
        .limit(20)
        .offset(params.page())
        .all()
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
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows: Vec<(i32, String, String)> = db
        .select((schema.user.id, schema.user.name, schema.user.email))
        .from(schema.user)
        .r#where(eq(schema.user.id, params.user_id(256)))
        .all()
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
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows: Vec<(i32, String, i32)> = db
        .select((schema.post.id, schema.post.title, schema.post.author_id))
        .from(schema.post)
        .limit(20)
        .offset(params.page())
        .all()
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
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows: Vec<(String, String)> = db
        .select((schema.user.name, schema.post.title))
        .from(schema.user)
        .inner_join((schema.post, eq(schema.user.id, schema.post.author_id)))
        .all()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|(name, title)| DetailRow { name, title })
            .collect(),
    ))
}
