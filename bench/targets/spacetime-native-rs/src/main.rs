//! SpacetimeDB native module HTTP wrapper (Rust).
//!
//! Connects to SpacetimeDB via WebSocket, subscribes to bench_users and
//! bench_posts tables, then serves the standard benchmark HTTP endpoints
//! from the local subscription cache.
//!
//! Data is seeded via SpacetimeDB's PGWire interface (same approach as all
//! other benchmark targets). The WebSocket subscription picks up the seeded
//! rows automatically.
//!
//! Prerequisites:
//!   1. Generate bindings:
//!      spacetime generate --lang rust \
//!      --out-dir bench/targets/spacetime-native-rs/src/module_bindings \
//!      --project-path bench/targets/spacetime-module
//!   2. Publish module:
//!      spacetime publish bench-module bench/targets/spacetime-module
//!
//! Environment variables:
//!   SPACETIME_URI      - WebSocket URI (default: ws://127.0.0.1:3000)
//!   SPACETIME_MODULE   - Module name (default: bench-module)
//!   SPACETIME_PG_HOST  - PGWire host (default: 127.0.0.1)
//!   SPACETIME_PG_PORT  - PGWire port (default: 5433)
//!   SPACETIME_TOKEN    - Identity token (or read from ~/.config/spacetime/cli.toml)
//!   BENCH_SEED         - Deterministic seed value
//!   BENCH_TRIAL        - Trial number

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::Arc;
use sysinfo::System;

// ---------------------------------------------------------------------------
// Seed data types (must match bench/spec/seed.schema.v1.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SeedData {
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    seed: u64,
    users: Vec<SeedUser>,
    posts: Vec<SeedPost>,
}

#[derive(Debug, Deserialize)]
struct SeedUser {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct SeedPost {
    title: String,
    body: String,
    author_id: i32,
}

fn load_seed_data() -> SeedData {
    if let Ok(path) = std::env::var("BENCH_SEED_FILE") {
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read seed file {path}: {e}"));
        serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("failed to parse seed file {path}: {e}"))
    } else {
        panic!("BENCH_SEED_FILE env var not set");
    }
}

mod module_bindings;

use module_bindings::DbConnection;
use module_bindings::bench_posts_table::BenchPostsTableAccess;
use module_bindings::bench_users_table::BenchUsersTableAccess;
use spacetimedb_sdk::{DbContext, Table};

// ---------------------------------------------------------------------------
// Shared types matching the benchmark contract
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct UserRow {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct PostRow {
    id: u32,
    title: String,
    author_id: u32,
}

#[derive(Debug, Serialize)]
struct DetailRow {
    name: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    id: Option<i32>,
    idx: Option<usize>,
}

impl QueryParams {
    fn page(&self) -> usize {
        self.idx.map(|i| i % 64).unwrap_or(0)
    }
    fn user_id(&self, n: i32) -> u32 {
        self.id
            .map(|i| i.rem_euclid(n).max(1) as u32)
            .unwrap_or(1)
    }
}

// ---------------------------------------------------------------------------
// App state: holds the SpacetimeDB connection for cache access
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    conn: Arc<DbConnection>,
}

// ---------------------------------------------------------------------------
// PGWire seeding (same approach as all other bench targets)
// ---------------------------------------------------------------------------

/// Build a `tokio_postgres::Config` for SpacetimeDB PGWire.
fn spacetime_pg_config() -> tokio_postgres::Config {
    let host = std::env::var("SPACETIME_PG_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("SPACETIME_PG_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5433);
    let dbname = std::env::var("SPACETIME_MODULE").unwrap_or_else(|_| "bench-module".to_string());
    let token = spacetime_token();

    let mut config = tokio_postgres::Config::new();
    config.host(&host);
    config.port(port);
    config.dbname(&dbname);
    config.user(&dbname);
    config.password(&token);
    config
}

/// Read the SpacetimeDB identity token for PGWire auth.
fn spacetime_token() -> String {
    if let Ok(tok) = std::env::var("SPACETIME_TOKEN")
        && !tok.trim().is_empty()
    {
        return tok;
    }

    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return String::new();
    }
    let path = std::path::Path::new(&home)
        .join(".config")
        .join("spacetime")
        .join("cli.toml");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("spacetimedb_token") {
            let val = val.trim();
            if let Some(val) = val.strip_prefix('=') {
                let val = val.trim().trim_matches('"').trim_matches('\'');
                if !val.is_empty() {
                    return val.to_string();
                }
            }
        }
    }

    String::new()
}

/// Seed data via PGWire SQL (DELETE + INSERT), same as spacetime_pg target.
async fn seed_via_pgwire(data: &SeedData) -> Result<(), Box<dyn std::error::Error>> {
    let config = spacetime_pg_config();
    eprintln!("spacetime-native-rs: connecting to PGWire for seeding...");

    let (client, connection) = config
        .connect(tokio_postgres::NoTls)
        .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("spacetime-native-rs: pgwire connection error: {e}");
        }
    });

    // Clear existing data
    client.simple_query("DELETE FROM bench_posts").await?;
    client.simple_query("DELETE FROM bench_users").await?;

    // Seed users one at a time (SpacetimeDB PGWire requires all columns including auto_inc id = 0)
    for user in &data.users {
        let name = sql_escape(&user.name);
        let email = sql_escape(&user.email);
        let sql = format!(
            "INSERT INTO bench_users (id, name, email) VALUES (0, '{name}', '{email}')"
        );
        client.simple_query(&sql).await?;
    }

    // Seed posts one at a time
    for post in &data.posts {
        let title = sql_escape(&post.title);
        let body = sql_escape(&post.body);
        let sql = format!(
            "INSERT INTO bench_posts (id, title, body, author_id) VALUES (0, '{title}', '{body}', {})",
            post.author_id
        );
        client.simple_query(&sql).await?;
    }

    eprintln!(
        "spacetime-native-rs: seeded {} users + {} posts via PGWire",
        data.users.len(),
        data.posts.len()
    );
    Ok(())
}

/// Escape single quotes for SQL string literals (Simple Query Protocol).
fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

// ---------------------------------------------------------------------------
// Handlers — query the local subscription cache
// ---------------------------------------------------------------------------

async fn stats(_: State<AppState>) -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    let cpu: Vec<f64> = sys.cpus().iter().map(|c| f64::from(c.cpu_usage())).collect();
    Json(if cpu.is_empty() { vec![0.0] } else { cpu })
}

async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<UserRow>>, StatusCode> {
    let offset = params.page();
    let mut users: Vec<_> = state.conn.db.bench_users().iter().collect();
    users.sort_by_key(|u| u.id);
    let rows: Vec<UserRow> = users
        .into_iter()
        .skip(offset)
        .take(20)
        .map(|u| UserRow {
            id: u.id,
            name: u.name.clone(),
            email: u.email.clone(),
        })
        .collect();
    Ok(Json(rows))
}

async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<UserRow>>, StatusCode> {
    let target_id = params.user_id(256);
    let rows: Vec<UserRow> = if let Some(u) = state.conn.db.bench_users().id().find(&target_id) {
        vec![UserRow {
            id: u.id,
            name: u.name.clone(),
            email: u.email.clone(),
        }]
    } else {
        vec![]
    };
    Ok(Json(rows))
}

async fn orders(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<PostRow>>, StatusCode> {
    let offset = params.page();
    let mut posts: Vec<_> = state.conn.db.bench_posts().iter().collect();
    posts.sort_by_key(|p| p.id);
    let rows: Vec<PostRow> = posts
        .into_iter()
        .skip(offset)
        .take(20)
        .map(|p| PostRow {
            id: p.id,
            title: p.title.clone(),
            author_id: p.author_id,
        })
        .collect();
    Ok(Json(rows))
}

async fn orders_with_details(
    State(state): State<AppState>,
    Query(_): Query<QueryParams>,
) -> Result<Json<Vec<DetailRow>>, StatusCode> {
    let users: std::collections::HashMap<u32, String> = state
        .conn
        .db
        .bench_users()
        .iter()
        .map(|u| (u.id, u.name.clone()))
        .collect();

    let rows: Vec<DetailRow> = state
        .conn
        .db
        .bench_posts()
        .iter()
        .filter_map(|p| {
            users.get(&p.author_id).map(|name: &String| DetailRow {
                name: name.clone(),
                title: p.title.clone(),
            })
        })
        .collect();
    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = std::env::var("SPACETIME_URI").unwrap_or_else(|_| "ws://127.0.0.1:3000".into());
    let module = std::env::var("SPACETIME_MODULE").unwrap_or_else(|_| "bench-module".into());

    // 1. Load pre-generated seed data and seed via PGWire SQL
    let data = load_seed_data();
    seed_via_pgwire(&data).await?;

    // 2. Connect to SpacetimeDB via WebSocket for subscription cache
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let ready_tx = std::sync::Mutex::new(Some(ready_tx));

    let conn = DbConnection::builder()
        .with_uri(&uri)
        .with_database_name(&module)
        .on_connect(move |_conn, _identity, _token| {
            if let Some(tx) = ready_tx.lock().unwrap().take() {
                let _ = tx.send(());
            }
        })
        .build()?;

    conn.run_threaded();

    ready_rx.await.map_err(|_| "connection callback never fired")?;

    // 3. Subscribe to benchmark tables (picks up PGWire-seeded data)
    conn.subscription_builder()
        .on_applied(|_ctx| {
            eprintln!("spacetime-native-rs: subscription applied");
        })
        .subscribe([
            "SELECT * FROM bench_users",
            "SELECT * FROM bench_posts",
        ]);

    // Wait for subscription to reflect seeded data
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let conn = Arc::new(conn);
    let state = AppState { conn };

    let app = Router::new()
        .route("/stats", get(stats))
        .route("/customers", get(customers))
        .route("/customer-by-id", get(customer_by_id))
        .route("/orders", get(orders))
        .route("/orders-with-details", get(orders_with_details))
        .with_state(state);

    // Bind ephemeral port
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;

    let server = tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .expect("server init")
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await
            .expect("server failed");
    });

    println!("LISTENING port={port}");

    server.await?;
    Ok(())
}
