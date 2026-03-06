use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use drizzle::core::expr::eq;
use drizzle::postgres::prelude::*;
use std::sync::mpsc;

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

/// Command dispatched from async handlers to the sync worker thread.
enum DbCmd {
    Customers {
        page: usize,
        reply: oneshot::Sender<Result<Vec<UserRow>, StatusCode>>,
    },
    CustomerById {
        id: i32,
        reply: oneshot::Sender<Result<Vec<UserRow>, StatusCode>>,
    },
    Orders {
        page: usize,
        reply: oneshot::Sender<Result<Vec<PostRow>, StatusCode>>,
    },
    Details {
        reply: oneshot::Sender<Result<Vec<DetailRow>, StatusCode>>,
    },
}

#[derive(Clone)]
struct AppState {
    tx: mpsc::Sender<DbCmd>,
}

pub async fn serve(seed: u64, trial: u32) -> Result<ServerHandle, Fail> {
    let (cmd_tx, cmd_rx) = mpsc::channel::<DbCmd>();
    let (ready_tx, ready_rx) = oneshot::channel::<Result<(), String>>();

    let worker = std::thread::spawn(move || {
        let mut conn = ::postgres::Client::connect(&pg_url(), ::postgres::NoTls)
            .map_err(|err| format!("postgres connect failed: {err}"))?;
        conn.batch_execute("DROP TABLE IF EXISTS bench_posts; DROP TABLE IF EXISTS bench_users;")
            .map_err(|err| format!("postgres drop failed: {err}"))?;
        let (mut db, schema) = drizzle::postgres::sync::Drizzle::new(conn, Schema::new());
        db.create()
            .map_err(|err| format!("postgres create failed: {err}"))?;
        db.insert(schema.user)
            .values(users!(seed, trial, 256))
            .execute()
            .map_err(|err| format!("postgres seed users failed: {err}"))?;
        db.insert(schema.post)
            .values(posts!(seed, trial, 1024, 256))
            .execute()
            .map_err(|err| format!("postgres seed posts failed: {err}"))?;

        let _ = ready_tx.send(Ok(()));

        let schema = Schema::new();
        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                DbCmd::Customers { page, reply } => {
                    let res = db
                        .select((schema.user.id, schema.user.name, schema.user.email))
                        .from(schema.user)
                        .limit(20)
                        .offset(page)
                        .all()
                        .map(|rows: Vec<(i32, String, String)>| {
                            rows.into_iter()
                                .map(|(id, name, email)| UserRow { id, name, email })
                                .collect()
                        })
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
                    let _ = reply.send(res);
                }
                DbCmd::CustomerById { id, reply } => {
                    let res = db
                        .select((schema.user.id, schema.user.name, schema.user.email))
                        .from(schema.user)
                        .r#where(eq(schema.user.id, id))
                        .all()
                        .map(|rows: Vec<(i32, String, String)>| {
                            rows.into_iter()
                                .map(|(id, name, email)| UserRow { id, name, email })
                                .collect()
                        })
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
                    let _ = reply.send(res);
                }
                DbCmd::Orders { page, reply } => {
                    let res = db
                        .select((schema.post.id, schema.post.title, schema.post.author_id))
                        .from(schema.post)
                        .limit(20)
                        .offset(page)
                        .all()
                        .map(|rows: Vec<(i32, String, i32)>| {
                            rows.into_iter()
                                .map(|(id, title, author_id)| PostRow {
                                    id,
                                    title,
                                    author_id,
                                })
                                .collect()
                        })
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
                    let _ = reply.send(res);
                }
                DbCmd::Details { reply } => {
                    let res = db
                        .select((schema.user.name, schema.post.title))
                        .from(schema.user)
                        .inner_join((schema.post, eq(schema.user.id, schema.post.author_id)))
                        .all()
                        .map(|rows: Vec<(String, String)>| {
                            rows.into_iter()
                                .map(|(name, title)| DetailRow { name, title })
                                .collect()
                        })
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
                    let _ = reply.send(res);
                }
            }
        }
        Ok(())
    });

    ready_rx
        .await
        .map_err(|_| Fail::new(Code::RunFail, "pg_sync worker dropped before ready"))?
        .map_err(|msg| Fail::new(Code::RunFail, msg))?;

    let router = Router::new()
        .route("/stats", get(stats))
        .route("/customers", get(customers))
        .route("/customer-by-id", get(customer_by_id))
        .route("/orders", get(orders))
        .route("/orders-with-details", get(orders_with_details))
        .with_state(AppState { tx: cmd_tx });
    let mut handle = spawn_server(router).await?;
    handle.workers.push(worker);
    Ok(handle)
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
    let (tx, rx) = oneshot::channel();
    state
        .tx
        .send(DbCmd::Customers {
            page: params.page(),
            reply: tx,
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??,
    ))
}

#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<UserRow>>, StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .tx
        .send(DbCmd::CustomerById {
            id: params.user_id(256),
            reply: tx,
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??,
    ))
}

#[debug_handler(state = AppState)]
async fn orders(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<PostRow>>, StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .tx
        .send(DbCmd::Orders {
            page: params.page(),
            reply: tx,
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??,
    ))
}

#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(_params): Query<QueryParams>,
) -> Result<Json<Vec<DetailRow>>, StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .tx
        .send(DbCmd::Details { reply: tx })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??,
    ))
}
