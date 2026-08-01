//! Environment plumbing shared by both toasty backends.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::process::Command;

pub type DynError = Box<dyn std::error::Error + Send + Sync>;

pub fn fail(msg: impl Into<String>) -> DynError {
    msg.into().into()
}

/// Async worker threads for the tokio runtime. The benchmark runner injects
/// `target.proc.workers` as `BENCH_WORKERS`; every spec declares `workers: 1`,
/// so the default matches the single-threaded Bun targets rather than silently
/// taking every core the way `#[tokio::main]` would.
pub fn configured_workers(default: usize) -> usize {
    std::env::var("BENCH_WORKERS")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|workers| *workers > 0)
        .unwrap_or(default)
}

pub fn configured_pool_size(default: usize) -> usize {
    std::env::var("BENCH_POOL_SIZE")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(default)
}

pub fn configured_seed(default: u64) -> u64 {
    std::env::var("BENCH_SEED")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub id: Option<i32>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub term: Option<String>,
}

impl QueryParams {
    pub fn limit_or(&self, default: usize) -> i64 {
        self.limit.unwrap_or(default) as i64
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0) as i64
    }

    pub fn id_mod(&self, n: i32) -> i32 {
        self.id.map(|i| (i - 1).rem_euclid(n) + 1).unwrap_or(1)
    }

    pub fn pattern(&self) -> String {
        format!("%{}%", self.term.as_deref().unwrap_or_default())
    }
}

pub fn normalize_database_url() -> String {
    let raw = std::env::var("DATABASE_URL").unwrap_or_default();
    if raw.trim().is_empty() {
        return "postgres://postgres:postgres@localhost:5432/drizzle_test".to_string();
    }
    if raw.starts_with("postgres://") || raw.starts_with("postgresql://") {
        return raw;
    }

    let mut parts = BTreeMap::new();
    for token in raw.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            parts.insert(key, value);
        }
    }
    let user = parts.get("user").copied().unwrap_or("postgres");
    let password = parts.get("password").copied().unwrap_or("postgres");
    let host = parts.get("host").copied().unwrap_or("localhost");
    let port = parts.get("port").copied().unwrap_or("5432");
    let dbname = parts.get("dbname").copied().unwrap_or("drizzle_test");
    format!("postgres://{user}:{password}@{host}:{port}/{dbname}")
}

/// Shell out to the benchmark runner's deterministic PostgreSQL seeder, exactly
/// like the other PostgreSQL ORM targets do.
pub fn seed_postgres(seed: u64) -> Result<(), DynError> {
    let seed = seed.to_string();
    let status = if let Ok(runner) = std::env::var("BENCH_RUNNER_BIN") {
        Command::new(runner)
            .args(["seed-postgres", "--seed", &seed])
            .status()?
    } else {
        Command::new("cargo")
            .args([
                "run",
                "-q",
                "--release",
                "-p",
                "bench-runner",
                "--",
                "seed-postgres",
                "--seed",
                &seed,
            ])
            .status()?
    };
    if status.success() {
        Ok(())
    } else {
        Err(fail(format!(
            "bench-runner seed-postgres exited with {status}"
        )))
    }
}
