//! Cloudflare Hyperdrive connector (async, WASM-only).
//!
//! Hyperdrive is Cloudflare's connection pooler and edge cache for existing
//! `PostgreSQL` databases. Inside a Worker, the binding hands out a
//! `worker::Socket` already connected to the pooler, and the pooler speaks the
//! plain `PostgreSQL` wire protocol.
//!
//! This module is **not a separate driver**. `worker::Socket` implements
//! tokio's `AsyncRead`/`AsyncWrite`, so
//! [`tokio_postgres::Config::connect_raw`] hands back the very same
//! [`tokio_postgres::Client`] the native driver wraps. Everything downstream —
//! query surface, transactions, savepoints, prepared statements and the
//! statement cache, relational queries, `migrate`, `push`, `introspect` — is
//! the [`tokio`](crate::postgres::tokio) driver compiled verbatim for
//! `wasm32-unknown-unknown`. The only thing this module adds is the dial.
//!
//! # Requirements
//!
//! - `target_arch = "wasm32"` — the binding only links inside a Worker runtime.
//! - The `worker` crate.
//!
//! ```toml
//! [dependencies]
//! drizzle = { version = "*", features = ["hyperdrive", "uuid"] }
//! worker = { version = "*" }
//! ```
//!
//! ```toml
//! # wrangler.toml
//! [[hyperdrive]]
//! binding = "HYPERDRIVE"
//! id = "<your-hyperdrive-id>"
//! ```
//!
//! # TLS
//!
//! Hyperdrive terminates TLS at the edge and the Worker reaches the pooler over
//! a local, already-authenticated channel, so the documented pattern is
//! [`NoTls`](tokio_postgres::NoTls) — which is what [`connect`] uses.
//!
//! Dialing a database *directly* (no Hyperdrive) with
//! `worker::Socket::builder()` does need TLS. That path is out of scope for
//! [`connect_raw`], which is also `NoTls`: use `worker`'s own
//! `postgres_tls::PassthroughTls` with `Config::connect_raw` (enable the
//! `worker` crate's `tokio-postgres` feature), then hand the resulting client
//! to [`Drizzle::new`](crate::postgres::tokio::Drizzle::new).
//!
//! # Quick start
//!
//! ```rust
//! # let _ = r####"
//! use drizzle::postgres::prelude::*;
//! use drizzle::postgres::hyperdrive;
//! use worker::{event, Context, Env, Request, Response};
//!
//! #[PostgresTable]
//! struct User {
//!     #[column(serial, primary)]
//!     id: i32,
//!     name: String,
//! }
//!
//! #[derive(PostgresSchema)]
//! struct AppSchema {
//!     user: User,
//! }
//!
//! #[event(fetch)]
//! async fn fetch(_req: Request, env: Env, _ctx: Context) -> worker::Result<Response> {
//!     let (db, AppSchema { user }) =
//!         hyperdrive::connect(&env.hyperdrive("HYPERDRIVE")?, AppSchema::new())
//!             .await
//!             .map_err(|e| worker::Error::RustError(e.to_string()))?;
//!
//!     db.insert(user)
//!         .values([InsertUser::new("Alice")])
//!         .execute()
//!         .await
//!         .map_err(|e| worker::Error::RustError(e.to_string()))?;
//!
//!     let users: Vec<SelectUser> = db
//!         .select(())
//!         .from(user)
//!         .all()
//!         .await
//!         .map_err(|e| worker::Error::RustError(e.to_string()))?;
//!
//!     Response::ok(format!("{} users", users.len()))
//! }
//! # "####;
//! ```
//!
//! # Migrations
//!
//! Prefer applying migrations out of band (CI, or `drizzle migrate` against the
//! database's direct connection string) — a Worker invocation is short-lived
//! and many run concurrently. When the Worker must migrate itself,
//! [`Drizzle::migrate`](crate::postgres::tokio::Drizzle::migrate) works
//! unchanged: it takes the same `pg_advisory_lock`, so concurrent invocations
//! serialize rather than race, and
//! [`migrate_with_repair`](crate::postgres::tokio::Drizzle::migrate_with_repair)
//! reconciles a migration interrupted by a Worker eviction.
//!
//! ```rust
//! # let _ = r####"
//! use drizzle_migrations::Tracking;
//!
//! // Embeds the migration files at compile time (expands to a Vec).
//! let migrations = drizzle::include_migrations!("./migrations");
//!
//! let (mut db, schema) = hyperdrive::connect(&env.hyperdrive("HYPERDRIVE")?, AppSchema::new()).await?;
//! db.migrate(&migrations, Tracking::POSTGRES).await?;
//! # "####;
//! ```
//!
//! `migrate` needs `&mut Drizzle` with no outstanding clones, so run it before
//! handing clones to other tasks.
//!
//! # Lifetime of the connection
//!
//! [`tokio_postgres`] splits a connection into a [`Client`] and a driver future
//! that owns the socket. The future is spawned with
//! [`wasm_bindgen_futures::spawn_local`], so it lives as long as the Worker
//! invocation that created it and is torn down with the isolate. A `Client`
//! therefore must not outlive the request that dialed it — connect per
//! invocation and let Hyperdrive's pooler absorb the cost.
//!
//! # Integer precision
//!
//! Unlike the D1 and Durable Objects drivers, values never cross a JS number
//! boundary here: only raw bytes traverse the socket and `postgres-types`
//! decodes the binary wire format in wasm. `i64`, `numeric`, and `bytea`
//! round-trip exactly.

use drizzle_core::error::DrizzleError;
use tokio_postgres::{Config, NoTls};
use worker::{Hyperdrive, Socket};

use crate::builder::postgres::tokio_postgres::Drizzle;

/// `tokio_postgres::Error`'s `Display` is just "db error"; the server's actual
/// message lives in the `DbError` source.
fn describe(error: &tokio_postgres::Error) -> String {
    error
        .as_db_error()
        .map_or_else(|| error.to_string(), ToString::to_string)
}

/// Connects to `PostgreSQL` through a Cloudflare Hyperdrive binding.
///
/// Returns the same `(Drizzle, Schema)` tuple as
/// [`Drizzle::new`](crate::postgres::tokio::Drizzle::new), for destructuring.
///
/// The connection string carried by the binding points at the local pooler
/// endpoint; TLS is terminated by Hyperdrive at the edge, so the wire to the
/// pooler is dialed with [`NoTls`].
///
/// # Errors
///
/// Returns [`DrizzleError::Other`] if the binding cannot open a socket, if its
/// connection string does not parse as a [`Config`], or if the `PostgreSQL`
/// startup handshake fails.
pub async fn connect<S: Copy>(
    hyperdrive: &Hyperdrive,
    schema: S,
) -> drizzle_core::error::Result<(Drizzle<S>, S)> {
    let socket = hyperdrive.connect().map_err(|e| {
        DrizzleError::Other(format!("hyperdrive: failed to open socket: {e}").into())
    })?;

    let config = hyperdrive
        .connection_string()
        .parse::<Config>()
        .map_err(|e| {
            DrizzleError::Other(format!("hyperdrive: invalid connection string: {e}").into())
        })?;

    connect_raw(&config, socket, schema).await
}

/// Connects over an already-opened [`Socket`] using an explicit [`Config`].
///
/// Use this when the socket does not come from a Hyperdrive binding — e.g. a
/// direct `worker::Socket::builder().connect(host, port)` — or when the
/// connection parameters need adjusting (`application_name`, `options`, a
/// different `dbname`) before the handshake.
///
/// The connection driver future is spawned with
/// [`wasm_bindgen_futures::spawn_local`]; if it ever resolves with an error the
/// error is written to the Worker console, since there is no join handle to
/// surface it through.
///
/// # Errors
///
/// Returns [`DrizzleError::Other`] if the `PostgreSQL` startup handshake fails.
pub async fn connect_raw<S: Copy>(
    config: &Config,
    socket: Socket,
    schema: S,
) -> drizzle_core::error::Result<(Drizzle<S>, S)> {
    let (client, connection) = config.connect_raw(socket, NoTls).await.map_err(|e| {
        DrizzleError::Other(format!("hyperdrive: connect failed: {}", describe(&e)).into())
    })?;

    // The driver future owns the socket and must be polled for the client to
    // make progress. Workers are single-threaded, so `spawn_local` (which does
    // not require `Send`) is the only option — and the right one: the task is
    // dropped with the isolate at the end of the invocation.
    worker::wasm_bindgen_futures::spawn_local(async move {
        if let Err(error) = connection.await {
            worker::console_error!(
                "hyperdrive: connection closed with error: {}",
                describe(&error)
            );
        }
    });

    Ok(Drizzle::new(client, schema))
}
