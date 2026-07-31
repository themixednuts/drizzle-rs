//! Toasty ORM benchmark targets (PostgreSQL and Turso).
//!
//! One binary, two backends, selected by the first CLI argument (or
//! `BENCH_TARGET_ID`), mirroring `bench/targets/rust-pg-orms`:
//!
//! * `toasty-pg` - connects to the shared PostgreSQL database (seeded by the
//!   benchmark runner) with toasty's `postgresql` driver.
//! * `toasty-turso` - creates a file-backed temp database, seeds it with the
//!   same drizzle-seed generator/seed/counts as the built-in Turso targets,
//!   then serves it with toasty's `turso` driver.
//!
//! Both backends honour `BENCH_WORKERS` (tokio worker threads, default 1),
//! `BENCH_POOL_SIZE` (connection pool, default 8) and `BENCH_SEED`, open every
//! pooled connection before printing `LISTENING`, and set `TCP_NODELAY` on
//! accepted sockets.

mod common;
mod pg;
mod seed_sqlite;
mod turso_backend;

use axum::serve::ListenerExt;
use axum::{Json, Router};
use common::{DynError, configured_seed, configured_workers, fail};
use std::io::Write as _;
use sysinfo::System;

fn main() -> Result<(), DynError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(configured_workers(1))
        .enable_all()
        .build()?;
    runtime.block_on(serve())
}

async fn serve() -> Result<(), DynError> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| std::env::var("BENCH_TARGET_ID").unwrap_or_default());
    let seed = configured_seed(42);

    match target.as_str() {
        "toasty-pg" => pg::serve(seed).await,
        "toasty-turso" => turso_backend::serve(seed).await,
        other => Err(fail(format!("unsupported toasty target: {other}"))),
    }
}

/// Per-core CPU usage, in the same shape every other target reports.
pub async fn stats() -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    let mut out: Vec<f64> = sys
        .cpus()
        .iter()
        .map(|cpu| f64::from(cpu.cpu_usage()))
        .collect();
    if out.is_empty() {
        out.push(0.0);
    }
    Json(out)
}

/// Bind an ephemeral port, disable Nagle on accepted sockets, announce
/// readiness, then serve. Called only after the pool is warm.
pub async fn run_server(app: Router) -> Result<(), DynError> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let listener = listener.tap_io(|stream| {
        // Small JSON bodies must not wait on Nagle's algorithm.
        let _ = stream.set_nodelay(true);
    });
    println!("LISTENING port={port}");
    std::io::stdout().flush()?;

    axum::serve(listener, app).await?;
    Ok(())
}
