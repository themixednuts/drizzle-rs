mod external;
pub(crate) use external::{resolve_arg_token, resolve_cmd_token};
mod pg_sync;
mod pg_tokio;
mod pool;
mod spacetime_pg;
mod sqlite;
mod turso;

use crate::cli::{Load, SeedPostgres, SeedSqlite, Serve, Suite};
use crate::clock::now_rfc3339;
use crate::code::{Code, Fail};
use crate::jsonio;
use crate::model::{Latency, PacingMode, Phase, Point, QueryPoint, RequestDoc, Stage, Workload};
use crate::stats::{avg, percentile_sorted};
use axum::Router;
use axum::serve::ListenerExt;
use serde::{Deserialize, Serialize};
use std::fmt::Write as FmtWrite;
use std::io::{BufRead, BufReader as StdBufReader, Write as IoWrite};
use std::net::TcpStream as StdTcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use tokio::net::TcpStream as TokioTcpStream;
use tokio::sync::{mpsc, oneshot};

// ---------------------------------------------------------------------------
// Northwind "micro" dataset sizes — shared by all benchmark targets.
// Each driver uses SeedConfig with these counts + the seed value to generate
// deterministic INSERT statements via drizzle-seed.
// ---------------------------------------------------------------------------

pub(crate) const SEED_CUSTOMERS: usize = 10_000;
pub(crate) const SEED_EMPLOYEES: usize = 200;
pub(crate) const SEED_ORDERS: usize = 50_000;
pub(crate) const SEED_SUPPLIERS: usize = 1_000;
pub(crate) const SEED_PRODUCTS: usize = 5_000;
pub(crate) const POSTGRES_POOL_SIZE: usize = 8;
pub(crate) const SQLITE_POOL_SIZE: usize = 8;
const DRIZZLE_BENCH_SLEEP_STEP_MS: u64 = 75;
// Details per order: controlled via SeedConfig::relation() (~6 per order, matching upstream avg)

pub(crate) fn configured_pool_size(default: usize) -> usize {
    std::env::var("BENCH_POOL_SIZE")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(default)
}

/// Worker threads the target server's tokio runtime must use.
///
/// The runner injects `BENCH_WORKERS` from `target.proc.workers` so that the
/// declared fairness metadata is the actual configuration. Defaults to 1: a
/// Rust target must not silently take every core when the spec claims one
/// worker, since the Bun/TS targets it is compared against are single-worker.
pub(crate) fn configured_workers() -> usize {
    std::env::var("BENCH_WORKERS")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|workers| *workers > 0)
        .unwrap_or(1)
}

/// Start the adapter server for the given target. Used by both `load` and `parity`.
///
/// Built-in targets dispatch to compiled Rust implementations.
/// External targets (indicated by `BENCH_SERVER_CMD` env var) are spawned as
/// child processes that must print `LISTENING port=<N>` to stdout.
pub(crate) async fn serve_target(target: &str, seed: u64) -> Result<ServerHandle, Fail> {
    // Check for external server command first
    let handle = if let Ok(cmd_json) = std::env::var("BENCH_SERVER_CMD") {
        let (mut handle, child) = external::serve(&cmd_json, target, seed).await?;
        handle.target_pid = Some(child.id());
        handle.external_child = Some(child);
        handle
    } else {
        serve_builtin_target(target, seed).await?
    };

    // `LISTENING` only proves the socket is bound. Pools may still be filling
    // and JIT tiers still cold; measuring before the target answers a real
    // request charges startup to the first bucket.
    await_healthy(handle.port, HEALTH_TIMEOUT).await?;
    Ok(handle)
}

const HEALTH_TIMEOUT: Duration = Duration::from_secs(120);

/// Poll `GET /stats` until it answers 200 or the deadline expires.
async fn await_healthy(port: u16, timeout: Duration) -> Result<(), Fail> {
    let deadline = Instant::now() + timeout;
    let mut last;
    loop {
        let probe = tokio::task::spawn_blocking(move || send_get_body(port, "/stats"))
            .await
            .map_err(|err| Fail::new(Code::TargetFail, format!("health probe panicked: {err}")))?;
        match probe {
            Ok((200, _)) => return Ok(()),
            Ok((status, _)) => last = format!("status {status}"),
            Err(err) => last = err,
        }
        if Instant::now() >= deadline {
            return Err(Fail::new(
                Code::TargetFail,
                format!("target did not become healthy on port {port}: {last}"),
            ));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn serve_builtin_target(target: &str, seed: u64) -> Result<ServerHandle, Fail> {
    match target {
        "drizzle-rs-sqlite" => sqlite::serve_select(seed).await,
        "drizzle-rs-sqlite-query" => sqlite::serve_query(seed).await,
        "rusqlite-sqlite" | "rusqlite-sqlite-prepared" => {
            sqlite::serve_raw_rusqlite_prepared(seed).await
        }
        "rusqlite-sqlite-unprepared" => sqlite::serve_raw_rusqlite_unprepared(seed).await,
        "drizzle-rs-pg-sync" => pg_sync::serve(seed).await,
        "drizzle-rs-pg" => pg_tokio::serve_select(seed).await,
        "drizzle-rs-pg-query" => pg_tokio::serve_query(seed).await,
        "tokio-postgres-unprepared" => pg_tokio::serve_unprepared(seed).await,
        "tokio-postgres-prepared" => pg_tokio::serve_prepared(seed).await,
        "spacetime-pgwire-rs" => spacetime_pg::serve(seed).await,
        "drizzle-rs-turso" => turso::serve(seed).await,
        "turso-sqlite-prepared" => turso::serve_raw_prepared(seed).await,
        "turso-sqlite-unprepared" => turso::serve_raw_unprepared(seed).await,
        other => Err(Fail::new(
            Code::InvalidCli,
            format!("unsupported target: {other}"),
        )),
    }
}

/// `GET /stats` — the liveness probe every target must answer.
///
/// The runner health-checks this route before measuring and the request mix
/// never includes it, so it must be cheap: sampling `System::new_all()` per
/// request cost ~200ms and made the probe the slowest route in the process.
/// One snapshot at first call is enough to answer with a well-formed body.
pub(crate) async fn stats() -> axum::Json<&'static [f64]> {
    axum::Json(cpu_snapshot())
}

/// Map any driver error to a 500.
///
/// The benchmark contract has no error body: a route either answers the
/// canonical JSON or it is an error the runner counts. Distinguishing failure
/// modes here would only add branches on the hot path.
pub(crate) fn db_err<E>(_: E) -> axum::http::StatusCode {
    axum::http::StatusCode::INTERNAL_SERVER_ERROR
}

// Alias tag for the recipient side of the `employees` self-join, shared by
// every driver that expresses that join through the builder.
drizzle_core::tag! { pub(crate) RecipientAlias, "r" }

/// `ilike` builds untyped raw SQL; the builder's `where` clause needs a
/// boolean-typed expression, so re-tag it without changing the SQL.
pub(crate) fn ilike_expr<'a, L>(
    left: L,
    pattern: &'a str,
) -> drizzle::core::expr::SQLExpr<
    'a,
    drizzle::postgres::values::PostgresValue<'a>,
    drizzle::postgres::types::Boolean,
>
where
    L: drizzle::core::ToSQL<'a, drizzle::postgres::values::PostgresValue<'a>>,
{
    drizzle::core::expr::SQLExpr::new(drizzle::postgres::expr::ilike(left, pattern))
}

/// Widen an expression to nullable while keeping its SQL type.
///
/// A `LEFT JOIN`'s right-hand columns are NULL on unmatched rows, but a column
/// ZST's own nullability says otherwise, so the decoded row type would have to
/// be the non-optional Rust type and would fail exactly on the rows the outer
/// join exists to produce.
pub(crate) fn nullable<'a, V, E>(
    expr: E,
) -> drizzle::core::expr::SQLExpr<'a, V, E::SQLType, drizzle::core::expr::Null, E::Aggregate>
where
    V: drizzle::core::SQLParam + 'a,
    E: drizzle::core::expr::Expr<'a, V>,
{
    drizzle::core::expr::SQLExpr::new(expr.into_expr_sql())
}

fn cpu_snapshot() -> &'static [f64] {
    static SNAPSHOT: std::sync::OnceLock<Vec<f64>> = std::sync::OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        let mut sys = System::new_all();
        sys.refresh_cpu_usage();
        let mut cpus = cpu_usage(&sys);
        if cpus.is_empty() {
            cpus.push(0.0);
        }
        cpus
    })
}

pub async fn serve(args: Serve) -> Result<Code, Fail> {
    let target = resolve_text(args.target, "BENCH_TARGET_ID", "--target")?;
    let seed: u64 = resolve_num(args.seed, "BENCH_SEED", "--seed")?;
    let _handle = serve_builtin_target(&target, seed).await?;

    println!("LISTENING port={}", _handle.port);
    std::io::stdout()
        .flush()
        .map_err(|err| Fail::new(Code::RunFail, format!("stdout flush failed: {err}")))?;

    let _: () = std::future::pending().await;
    Ok(Code::Success)
}

/// Dataset seed for the `seed-*` subcommands: the flag wins, then the
/// environment the runner injects into external targets, then the contract
/// default.
fn seed_or_default(explicit: Option<u64>) -> u64 {
    explicit
        .or_else(|| {
            std::env::var("BENCH_SEED")
                .ok()
                .and_then(|raw| raw.parse().ok())
        })
        .unwrap_or(42)
}

/// Build and seed a SQLite database file for an external target.
///
/// The TypeScript SQLite targets cannot run drizzle-seed themselves, so they
/// shell out here before announcing `LISTENING`. The rows are therefore the
/// same ones the built-in rusqlite targets serve.
pub(crate) async fn seed_sqlite(args: SeedSqlite) -> Result<Code, Fail> {
    let seed = seed_or_default(args.seed);
    let db = args.db;
    tokio::task::spawn_blocking(move || sqlite::create_and_seed(&db, seed))
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("sqlite seed panicked: {err}")))??;
    Ok(Code::Success)
}

pub(crate) async fn seed_postgres(args: SeedPostgres) -> Result<Code, Fail> {
    let seed = seed_or_default(args.seed);
    let database_url = pg_url();
    tokio::task::spawn_blocking(move || pg_sync::seed_database_url(&database_url, seed))
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("postgres seed panicked: {err}")))?
        .map_err(|msg| Fail::new(Code::RunFail, format!("postgres seed failed: {msg}")))?;
    Ok(Code::Success)
}

/// Send an HTTP GET and return the full response body (or error).
pub(crate) fn send_get_body(port: u16, path: &str) -> Result<(u16, String), String> {
    let mut stream = StdTcpStream::connect(("127.0.0.1", port))
        .map_err(|err| format!("connect failed: {err}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| format!("set_read_timeout failed: {err}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| format!("set_write_timeout failed: {err}"))?;

    let raw = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream
        .write_all(raw.as_bytes())
        .map_err(|err| format!("write failed: {err}"))?;

    let mut reader = StdBufReader::new(stream);

    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|err| format!("read status failed: {err}"))?;
    let status_code = parse_status_code(&status_line).unwrap_or(0);

    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|err| format!("read header failed: {err}"))?;
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(value) = header_value(line, "content-length") {
            content_length = value.parse().ok();
        }
    }

    let mut body = String::new();
    if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        std::io::Read::read_exact(&mut reader, &mut buf)
            .map_err(|err| format!("read body failed: {err}"))?;
        body = String::from_utf8_lossy(&buf).into_owned();
    } else {
        // `Connection: close` was requested, so the body runs to EOF.
        std::io::Read::read_to_string(&mut reader, &mut body)
            .map_err(|err| format!("read body failed: {err}"))?;
    }

    Ok((status_code, body))
}

pub async fn run(args: Load) -> Result<Code, Fail> {
    crate::proc::pin_self_to_load_cpuset();
    let out = resolve_path(args.out, "BENCH_TIMESERIES_OUT", "--out")?;
    let target = resolve_text(args.target, "BENCH_TARGET_ID", "--target")?;
    let trial: u32 = resolve_num(args.trial, "BENCH_TRIAL", "--trial")?;
    let seed: u64 = resolve_num(args.seed, "BENCH_SEED", "--seed")?;
    let suite = resolve_suite(args.suite, "BENCH_SUITE")?;
    let workload_path = resolve_path(args.workload, "BENCH_WORKLOAD_FILE", "--workload")?;
    let requests_path = resolve_path(args.requests, "BENCH_REQUESTS_FILE", "--requests")?;

    let workload = jsonio::read::<Workload>(&workload_path, Code::InvalidInput)?;
    if workload.suite != suite.as_str() {
        return Err(Fail::new(
            Code::InvalidInput,
            format!(
                "load suite mismatch: cli={}, file={}",
                suite.as_str(),
                workload.suite
            ),
        ));
    }
    let requests = jsonio::read::<Vec<RequestDoc>>(&requests_path, Code::InvalidInput)?;
    if requests.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            "load requires a non-empty requests file",
        ));
    }

    if !workload.load.executor.ends_with("-vus") {
        return Err(Fail::new(
            Code::InvalidInput,
            format!(
                "workload.load.executor {} is not implemented; use constant-vus or ramping-vus",
                workload.load.executor
            ),
        ));
    }

    let handle = serve_target(&target, seed).await?;

    let port = handle.port;
    let target_pid = handle.target_pid;
    let measured = measure_vus_async(&workload, &requests, port, target_pid, seed, trial).await;

    // Always tear the target down, even when measurement failed, so the next
    // trial does not race a still-listening server.
    let shutdown = handle.shutdown().await;
    let measured = measured?;
    shutdown?;

    jsonio::write(out, &measured.series, Code::RunFail)?;
    // The trial aggregate is computed here, where the raw per-request samples
    // still exist: exact hold-phase percentiles cannot be recovered from the
    // per-bucket summaries alone.
    if let Ok(point_out) = std::env::var("BENCH_POINT_OUT") {
        jsonio::write(PathBuf::from(point_out), &measured.aggregate, Code::RunFail)?;
    }
    Ok(Code::Success)
}

// ---------------------------------------------------------------------------
// Server handle (owns a spawned axum task + optional worker threads)
// ---------------------------------------------------------------------------

pub(crate) struct ServerHandle {
    pub(crate) port: u16,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<(), String>>>,
    workers: Vec<std::thread::JoinHandle<Result<(), String>>>,
    pub(crate) temp_dirs: Vec<tempfile::TempDir>,
    pub(crate) external_child: Option<std::process::Child>,
    pub(crate) target_pid: Option<u32>,
}

impl ServerHandle {
    pub(crate) async fn shutdown(mut self) -> Result<(), Fail> {
        // Kill the external child's whole process group first: `bun run x`,
        // `cargo run`, and shell wrappers all leave the real server alive if
        // only the direct child is signalled.
        if let Some(mut child) = self.external_child.take() {
            tokio::task::spawn_blocking(move || crate::proc::kill_tree(&mut child))
                .await
                .map_err(|err| {
                    Fail::new(Code::RunFail, format!("target teardown panicked: {err}"))
                })?;
        }

        if let Some(tx) = self.stop.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(msg)) => return Err(Fail::new(Code::RunFail, msg)),
                Err(err) => {
                    return Err(Fail::new(
                        Code::RunFail,
                        format!("server task panicked: {err}"),
                    ));
                }
            }
        }
        for jh in self.workers {
            let res = tokio::task::spawn_blocking(|| jh.join())
                .await
                .map_err(|err| Fail::new(Code::RunFail, format!("worker join panicked: {err}")))?;
            match res {
                Ok(Ok(())) => {}
                Ok(Err(msg)) => return Err(Fail::new(Code::RunFail, msg)),
                Err(_) => return Err(Fail::new(Code::RunFail, "worker thread panicked")),
            }
        }
        self.temp_dirs.clear();
        Ok(())
    }
}

/// Bind an ephemeral port and spawn `app` as a tokio task.
async fn spawn_server(app: Router) -> Result<ServerHandle, Fail> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("bind failed: {err}")))?;
    let port = listener
        .local_addr()
        .map_err(|err| Fail::new(Code::RunFail, format!("local_addr failed: {err}")))?
        .port();
    // Nagle batching would add up to 40ms to small JSON responses and swamp the
    // latency signal we are trying to measure.
    let listener = listener.tap_io(|stream| {
        let _ = stream.set_nodelay(true);
    });

    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stop_rx.await;
            })
            .await
            .map_err(|err| format!("server failed: {err}"))
    });

    Ok(ServerHandle {
        port,
        stop: Some(stop_tx),
        task: Some(task),
        workers: Vec::new(),
        temp_dirs: Vec::new(),
        external_child: None,
        target_pid: Some(std::process::id()),
    })
}

// ---------------------------------------------------------------------------
// Measurement loop — concurrent workers, each with a keep-alive connection
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct RequestPlan {
    path: String,
    query_idx: usize,
}

#[derive(Clone)]
struct QueryKey {
    method: String,
    path: String,
}

#[derive(Clone, Default)]
struct QueryBucket {
    latencies: Vec<f64>,
    errors: u64,
    total: u64,
}

impl QueryBucket {
    fn record(&mut self, ok: bool, latency_ms: f64) {
        self.total += 1;
        if ok {
            self.latencies.push(latency_ms);
        } else {
            self.errors += 1;
        }
    }

    fn merge(&mut self, other: QueryBucket) {
        self.latencies.extend(other.latencies);
        self.errors += other.errors;
        self.total += other.total;
    }
}

/// One second of the load profile, tagged with the stage it came from and the
/// role that second plays in the measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SecondPlan {
    vus: u32,
    stage: u32,
    phase: Phase,
}

/// One completed request, handed from a VU task to the bucket collector.
struct Sample {
    query_idx: usize,
    ok: bool,
    latency_ms: f64,
}

/// Everything one trial produces: the per-bucket series for charts, plus the
/// trial aggregate computed from the merged raw samples while they still exist.
pub(crate) struct Measured {
    pub(crate) series: Vec<Point>,
    pub(crate) aggregate: Point,
}

/// Requests are drawn `plans[(vu * PLAN_STRIDE + iter) % len]`. The pool is
/// already shuffled, so a prime stride simply gives every VU its own starting
/// offset while preserving the endpoint mix.
const PLAN_STRIDE: u64 = 7_919;

async fn measure_vus_async(
    workload: &Workload,
    requests: &[RequestDoc],
    port: u16,
    target_pid: Option<u32>,
    seed: u64,
    trial: u32,
) -> Result<Measured, Fail> {
    let ramping = workload.load.executor.starts_with("ramping");
    let schedule = build_schedule(&workload.stages, ramping, workload.warmup_seconds());
    if schedule.is_empty() {
        return Err(Fail::new(Code::RunFail, "empty VU schedule"));
    }
    // A workload shorter than its own warmup (smoke tests, 1s previews) has no
    // steady state to restrict to; fall back to counting every bucket rather
    // than reporting an empty aggregate.
    let has_hold = schedule.iter().any(|plan| plan.phase == Phase::Hold);
    let counts_toward_aggregate = |phase: Phase| -> bool { !has_hold || phase == Phase::Hold };

    let bucket_s = workload.sampling.bucket_s.max(1) as usize;
    let (mut plans, query_keys) = request_plan(requests);
    // Decorrelate trials: without this every trial replays the same request
    // order, so any cache-friendly accident in the pool is measured five times
    // instead of averaged out.
    rotate_for_trial(&mut plans, seed, trial);
    let plans = Arc::new(plans);
    let query_keys = Arc::new(query_keys);
    let running = Arc::new(AtomicBool::new(true));
    let err_seen = Arc::new(AtomicBool::new(false));
    let first_err: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let (tx, mut rx) = mpsc::unbounded_channel::<Sample>();
    let pacing = workload.pacing.mode;

    // CPU/memory sampling lives on its own OS thread: run inline on the tokio
    // runtime it competes with the VU tasks it is supposed to be observing, and
    // the sampling cadence collapses to "whenever a bucket ends".
    let sampler = SysSampler::spawn(u64::from(workload.sampling.cpu_ms), target_pid);

    let mut workers: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let mut series: Vec<Point> = Vec::new();

    let mut agg_queries: Vec<QueryBucket> = vec![QueryBucket::default(); query_keys.len()];
    let mut agg_requests = 0_u64;
    let mut agg_errors = 0_u64;
    let mut agg_wall = 0.0_f64;
    let mut agg_cpu: Vec<f64> = Vec::new();
    let mut agg_cpu_samples = 0_usize;
    let mut agg_mem: Vec<f64> = Vec::new();

    let mut idx = 0_usize;
    while idx < schedule.len() {
        let plan = schedule[idx];
        // A bucket never straddles a stage or phase boundary, so every emitted
        // point carries one unambiguous (stage, phase) tag.
        let mut end = idx + 1;
        while end < schedule.len()
            && end - idx < bucket_s
            && schedule[end].stage == plan.stage
            && schedule[end].phase == plan.phase
        {
            end += 1;
        }

        let target_vus = schedule[idx..end]
            .iter()
            .map(|slot| slot.vus)
            .max()
            .unwrap_or(0) as usize;

        // The VU pool only grows; ramp-down stages are not honoured.
        while workers.len() < target_vus {
            let vu_id = workers.len() as u64;
            let plans = Arc::clone(&plans);
            let is_running = Arc::clone(&running);
            let err_seen = Arc::clone(&err_seen);
            let first_err = Arc::clone(&first_err);
            let sender = tx.clone();
            workers.push(tokio::spawn(async move {
                let mut conn = AsyncHttpConn::new(port);
                let mut iter = 0_u64;
                while is_running.load(Ordering::Relaxed) {
                    let plan = &plans[plan_index(vu_id, iter, plans.len())];

                    let t0 = Instant::now();
                    let outcome = conn.get(&plan.path).await;
                    let latency_ms = t0.elapsed().as_secs_f64() * 1000.0;
                    let ok = match outcome {
                        Ok(()) => true,
                        Err(msg) => {
                            if !err_seen.swap(true, Ordering::Relaxed)
                                && let Ok(mut slot) = first_err.lock()
                            {
                                *slot = Some(msg);
                            }
                            false
                        }
                    };

                    if sender
                        .send(Sample {
                            query_idx: plan.query_idx,
                            ok,
                            latency_ms,
                        })
                        .is_err()
                    {
                        break;
                    }

                    iter = iter.wrapping_add(1);
                    if pacing == PacingMode::DrizzleBenchmark {
                        // Per-VU counter, not a global one: derived from a shared
                        // atomic the sleep length tracks scheduling order rather
                        // than the VU's own progress, which turns the intended
                        // 0/75/.../375ms staircase into an artifact of contention.
                        let sleep_ms = (iter % 6) * DRIZZLE_BENCH_SLEEP_STEP_MS;
                        if sleep_ms > 0 {
                            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                        }
                    }
                }
            }));
        }

        let secs = (end - idx) as u64;
        let start = Instant::now();
        tokio::time::sleep(Duration::from_secs(secs)).await;
        let wall = start.elapsed().as_secs_f64().max(0.001);

        // Snapshot the queue depth before draining. Requests that land while we
        // drain completed after the window closed and belong to the next bucket;
        // counting them here inflates this bucket's rps against its own wall.
        let pending = rx.len();
        let mut latencies = Vec::with_capacity(pending);
        let mut errors = 0_u64;
        let mut total = 0_u64;
        let mut queries = vec![QueryBucket::default(); query_keys.len()];
        for _ in 0..pending {
            let Ok(sample) = rx.try_recv() else {
                break;
            };
            total += 1;
            if sample.ok {
                latencies.push(sample.latency_ms);
            } else {
                errors += 1;
            }
            queries[sample.query_idx].record(sample.ok, sample.latency_ms);
        }

        let snapshot = sampler.latest();
        let point = Point {
            time: now_rfc3339(),
            rps: total as f64 / wall,
            err: if total == 0 {
                0.0
            } else {
                errors as f64 / total as f64
            },
            latency: summarize_latency(&latencies),
            cpu: snapshot.cpu.clone(),
            mem_mb: snapshot.mem_mb,
            trial: Some(trial),
            stage: Some(plan.stage),
            phase: Some(plan.phase),
            requests: Some(total),
            queries: query_points(&query_keys, &queries, wall),
        };

        if counts_toward_aggregate(plan.phase) {
            agg_requests += total;
            agg_errors += errors;
            agg_wall += wall;
            for (slot, bucket) in agg_queries.iter_mut().zip(queries) {
                slot.merge(bucket);
            }
            if agg_cpu.len() < snapshot.cpu.len() {
                agg_cpu.resize(snapshot.cpu.len(), 0.0);
            }
            for (slot, value) in agg_cpu.iter_mut().zip(&snapshot.cpu) {
                *slot += value;
            }
            agg_cpu_samples += 1;
            if let Some(mem) = snapshot.mem_mb {
                agg_mem.push(mem);
            }
        }

        series.push(point);
        idx = end;
    }

    running.store(false, Ordering::Relaxed);
    drop(tx);
    for worker in workers {
        let _ = worker.await;
    }
    sampler.stop();

    if agg_requests > 0 && agg_errors == agg_requests {
        let msg = first_err
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_else(|| "all requests failed".to_string());
        return Err(Fail::new(Code::RunFail, msg));
    }

    // Exact percentiles over every steady-state sample of the trial. A median of
    // per-second p95s is not the trial p95 — it systematically hides the tail.
    let mut merged: Vec<f64> = Vec::with_capacity(
        agg_queries
            .iter()
            .map(|bucket| bucket.latencies.len())
            .sum(),
    );
    for bucket in &agg_queries {
        merged.extend_from_slice(&bucket.latencies);
    }

    let agg_wall = agg_wall.max(0.001);
    if agg_cpu_samples > 0 {
        for slot in &mut agg_cpu {
            *slot /= agg_cpu_samples as f64;
        }
    }
    if agg_cpu.is_empty() {
        agg_cpu.push(0.0);
    }

    let aggregate = Point {
        time: now_rfc3339(),
        rps: agg_requests as f64 / agg_wall,
        err: if agg_requests == 0 {
            0.0
        } else {
            agg_errors as f64 / agg_requests as f64
        },
        latency: summarize_latency(&merged),
        cpu: agg_cpu,
        mem_mb: if agg_mem.is_empty() {
            None
        } else {
            Some(avg(&agg_mem))
        },
        trial: Some(trial),
        stage: None,
        phase: None,
        requests: Some(agg_requests),
        queries: query_points(&query_keys, &agg_queries, agg_wall),
    };

    Ok(Measured { series, aggregate })
}

fn plan_index(vu_id: u64, iter: u64, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (vu_id.wrapping_mul(PLAN_STRIDE).wrapping_add(iter) % len as u64) as usize
}

/// Rotate the request plan by a deterministic per-(seed, trial) offset.
fn rotate_for_trial(plans: &mut [RequestPlan], seed: u64, trial: u32) {
    if plans.len() < 2 {
        return;
    }
    let mut mixed = seed ^ u64::from(trial).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mixed ^= mixed >> 33;
    mixed = mixed.wrapping_mul(0xff51_afd7_ed55_8ccd);
    mixed ^= mixed >> 33;
    mixed = mixed.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    mixed ^= mixed >> 33;
    let shift = (mixed % plans.len() as u64) as usize;
    plans.rotate_left(shift);
}

/// Build a per-second plan from the stage list.
///
/// `ramping` executors interpolate toward a stage's VU target; `constant-vus`
/// holds at the declared count for the whole stage. The first `warmup_s`
/// seconds of the run are retagged as warmup regardless of stage shape.
fn build_schedule(stages: &[Stage], ramping: bool, warmup_s: u32) -> Vec<SecondPlan> {
    let mut schedule = Vec::new();
    let mut prev = 0_u32;
    for (stage_idx, stage) in stages.iter().enumerate() {
        if stage.sec == 0 {
            continue;
        }
        let target = stage.vus.unwrap_or(prev);
        let stage_idx = stage_idx as u32;
        let phase = if ramping && target != prev {
            Phase::Ramp
        } else {
            Phase::Hold
        };

        if phase == Phase::Ramp && stage.sec > 1 {
            for i in 0..stage.sec {
                // k6 ramps continuously; each one-second bucket approximates the
                // VU count reached by the end of that bucket, not the prior count.
                let t = f64::from(i + 1) / f64::from(stage.sec);
                let vus = f64::from(prev) + (f64::from(target) - f64::from(prev)) * t;
                schedule.push(SecondPlan {
                    vus: vus.round() as u32,
                    stage: stage_idx,
                    phase,
                });
            }
        } else {
            for _ in 0..stage.sec {
                schedule.push(SecondPlan {
                    vus: target,
                    stage: stage_idx,
                    phase,
                });
            }
        }
        prev = target;
    }

    for slot in schedule.iter_mut().take(warmup_s as usize) {
        slot.phase = Phase::Warmup;
    }
    schedule
}

// ---------------------------------------------------------------------------
// Host metrics sampler
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct SysSnapshot {
    cpu: Vec<f64>,
    mem_mb: Option<f64>,
}

/// Samples host CPU and target process-tree memory on a dedicated OS thread at
/// the workload's `sampling.cpu_ms` cadence. Buckets read the latest snapshot
/// instead of refreshing sysinfo inline.
struct SysSampler {
    latest: Arc<Mutex<SysSnapshot>>,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl SysSampler {
    fn spawn(cpu_ms: u64, target_pid: Option<u32>) -> Self {
        let latest = Arc::new(Mutex::new(SysSnapshot::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let latest = Arc::clone(&latest);
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("bench-sysinfo".to_string())
                .spawn(move || {
                    let mut sys = System::new_all();
                    sys.refresh_cpu_usage();
                    let pid = target_pid.map(Pid::from_u32);
                    // sysinfo needs a minimum gap between CPU refreshes to
                    // produce a meaningful delta.
                    let interval = Duration::from_millis(cpu_ms.max(100));
                    while !stop.load(Ordering::Relaxed) {
                        std::thread::sleep(interval);
                        sys.refresh_cpu_usage();
                        let mem_mb = pid.and_then(|p| process_tree_memory_mb(&mut sys, p));
                        let cpu = cpu_usage(&sys);
                        if let Ok(mut slot) = latest.lock() {
                            *slot = SysSnapshot { cpu, mem_mb };
                        }
                    }
                })
                .ok()
        };
        Self {
            latest,
            stop,
            handle,
        }
    }

    fn latest(&self) -> SysSnapshot {
        let mut snapshot = self
            .latest
            .lock()
            .ok()
            .map(|slot| slot.clone())
            .unwrap_or_default();
        if snapshot.cpu.is_empty() {
            snapshot.cpu.push(0.0);
        }
        snapshot
    }

    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct AsyncHttpConn {
    port: u16,
    reader: Option<AsyncBufReader<TokioTcpStream>>,
    header_buf: String,
    request_buf: String,
}

impl AsyncHttpConn {
    fn new(port: u16) -> Self {
        Self {
            port,
            reader: None,
            header_buf: String::new(),
            request_buf: String::with_capacity(256),
        }
    }

    async fn connect(&mut self) -> Result<(), String> {
        let stream = tokio::time::timeout(
            Duration::from_secs(30),
            TokioTcpStream::connect(("127.0.0.1", self.port)),
        )
        .await
        .map_err(|_| "connect timeout".to_string())?
        .map_err(|err| format!("connect failed: {err}"))?;
        stream
            .set_nodelay(true)
            .map_err(|err| format!("set_nodelay: {err}"))?;
        self.reader = Some(AsyncBufReader::new(stream));
        Ok(())
    }

    /// Perform one request.
    ///
    /// Semantics that matter for measurement honesty:
    /// - a non-2xx response is *one* error; the body is drained and the
    ///   keep-alive connection is kept. Tearing the connection down and
    ///   retrying would fire a deterministic 404/500 twice, double-count it,
    ///   fold both attempts into one latency sample, and add connection churn
    ///   the target never actually caused.
    /// - a transport error invalidates the connection but the request is *not*
    ///   re-sent: a retry would hide the failure from the error rate.
    async fn get(&mut self, path: &str) -> Result<(), String> {
        if self.reader.is_none() {
            self.connect().await?;
        }

        let response = match self.send_and_read(path).await {
            Ok(response) => response,
            Err(err) => {
                self.reader = None;
                return Err(err);
            }
        };

        if !response.keep_alive {
            self.reader = None;
        }

        if response.ok {
            Ok(())
        } else {
            Err(format!("request failed: {}", response.status_line))
        }
    }

    async fn send_and_read(&mut self, path: &str) -> Result<HttpResponse, String> {
        let reader = self.reader.as_mut().ok_or("no connection")?;
        self.request_buf.clear();
        write!(
            &mut self.request_buf,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: keep-alive\r\n\r\n"
        )
        .map_err(|err| format!("format request: {err}"))?;
        tokio::time::timeout(
            Duration::from_secs(30),
            reader.get_mut().write_all(self.request_buf.as_bytes()),
        )
        .await
        .map_err(|_| "write timeout".to_string())?
        .map_err(|err| format!("write failed: {err}"))?;

        self.header_buf.clear();
        let read = tokio::time::timeout(
            Duration::from_secs(30),
            reader.read_line(&mut self.header_buf),
        )
        .await
        .map_err(|_| "read status timeout".to_string())?
        .map_err(|err| format!("read status: {err}"))?;
        if read == 0 {
            return Err("read status: eof".to_string());
        }
        let status = parse_status_code(&self.header_buf);
        let ok = status.is_some_and(|code| (200..300).contains(&code));
        let status_line = self.header_buf.trim().to_string();

        let mut content_length: Option<usize> = None;
        let mut chunked = false;
        let mut close = false;
        loop {
            self.header_buf.clear();
            let read = tokio::time::timeout(
                Duration::from_secs(30),
                reader.read_line(&mut self.header_buf),
            )
            .await
            .map_err(|_| "read header timeout".to_string())?
            .map_err(|err| format!("read header: {err}"))?;
            if read == 0 {
                return Err("read header: eof".to_string());
            }
            let line = self.header_buf.trim();
            if line.is_empty() {
                break;
            }
            if let Some(val) = header_value(line, "content-length") {
                content_length = val.parse().ok();
            } else if header_has_token(line, "transfer-encoding", "chunked") {
                chunked = true;
            } else if header_has_token(line, "connection", "close") {
                close = true;
            }
        }

        // 204/304 never carry a body, regardless of framing headers.
        let bodyless = matches!(status, Some(204) | Some(304));
        let mut eof_delimited = false;

        if bodyless {
            // Nothing to drain.
        } else if let Some(len) = content_length {
            let mut remaining = len;
            let mut discard = [0u8; 8192];
            while remaining > 0 {
                let to_read = remaining.min(discard.len());
                let n = tokio::time::timeout(
                    Duration::from_secs(30),
                    reader.read(&mut discard[..to_read]),
                )
                .await
                .map_err(|_| "read body timeout".to_string())?
                .map_err(|err| format!("read body: {err}"))?;
                if n == 0 {
                    return Err("read body: eof".to_string());
                }
                remaining -= n;
            }
        } else if chunked {
            loop {
                self.header_buf.clear();
                let read = tokio::time::timeout(
                    Duration::from_secs(30),
                    reader.read_line(&mut self.header_buf),
                )
                .await
                .map_err(|_| "read chunk size timeout".to_string())?
                .map_err(|err| format!("read chunk size: {err}"))?;
                if read == 0 {
                    return Err("read chunk size: eof".to_string());
                }
                let size = usize::from_str_radix(self.header_buf.trim(), 16).unwrap_or(0);
                if size == 0 {
                    self.header_buf.clear();
                    let _ = reader.read_line(&mut self.header_buf).await;
                    break;
                }
                let mut remaining = size;
                let mut discard = [0u8; 8192];
                while remaining > 0 {
                    let to_read = remaining.min(discard.len());
                    let n = tokio::time::timeout(
                        Duration::from_secs(30),
                        reader.read(&mut discard[..to_read]),
                    )
                    .await
                    .map_err(|_| "read chunk timeout".to_string())?
                    .map_err(|err| format!("read chunk: {err}"))?;
                    if n == 0 {
                        return Err("read chunk: eof".to_string());
                    }
                    remaining -= n;
                }
                self.header_buf.clear();
                let _ = reader.read_line(&mut self.header_buf).await;
            }
        } else {
            // No framing headers: the body runs until the peer closes. Drain to
            // EOF and mark the connection dead, otherwise the next request would
            // read this body's tail as a status line.
            eof_delimited = true;
            let mut discard = [0u8; 8192];
            loop {
                let n = tokio::time::timeout(Duration::from_secs(30), reader.read(&mut discard))
                    .await
                    .map_err(|_| "read body timeout".to_string())?
                    .map_err(|err| format!("read body: {err}"))?;
                if n == 0 {
                    break;
                }
            }
        }

        Ok(HttpResponse {
            ok,
            status_line,
            keep_alive: !close && !eof_delimited,
        })
    }
}

struct HttpResponse {
    ok: bool,
    status_line: String,
    keep_alive: bool,
}

/// Extract the status code from an HTTP/1.x status line. Any HTTP/1.x version
/// is accepted: pinning to `HTTP/1.1 200` misclassifies a perfectly good
/// `HTTP/1.0 200` or `HTTP/1.1 201` as a failure.
fn parse_status_code(line: &str) -> Option<u16> {
    let mut parts = line.split_whitespace();
    let version = parts.next()?;
    if !version.starts_with("HTTP/1.") {
        return None;
    }
    parts.next()?.parse::<u16>().ok()
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn request_plan(requests: &[RequestDoc]) -> (Vec<RequestPlan>, Vec<QueryKey>) {
    let mut query_keys = Vec::<QueryKey>::new();
    let mut query_indexes = std::collections::BTreeMap::<(String, String), usize>::new();
    let plans = requests
        .iter()
        .map(|request| {
            let method = request.method.to_ascii_uppercase();
            let route = request.path.clone();
            let query_idx = *query_indexes
                .entry((method.clone(), route.clone()))
                .or_insert_with(|| {
                    let idx = query_keys.len();
                    query_keys.push(QueryKey {
                        method,
                        path: route,
                    });
                    idx
                });
            RequestPlan {
                path: build_path(request),
                query_idx,
            }
        })
        .collect();

    (plans, query_keys)
}

fn query_points(keys: &[QueryKey], buckets: &[QueryBucket], wall: f64) -> Vec<QueryPoint> {
    keys.iter()
        .zip(buckets)
        .filter(|(_, bucket)| bucket.total > 0)
        .map(|(key, bucket)| QueryPoint {
            method: key.method.clone(),
            path: key.path.clone(),
            rps: bucket.total as f64 / wall,
            err: bucket.errors as f64 / bucket.total as f64,
            latency: summarize_latency(&bucket.latencies),
        })
        .collect()
}

fn header_value<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let (key, value) = line.split_once(':')?;
    key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
}

fn header_has_token(line: &str, name: &str, token: &str) -> bool {
    header_value(line, name)
        .map(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case(token))
        })
        .unwrap_or(false)
}

fn build_path(req: &RequestDoc) -> String {
    let mut path = req.path.clone();
    if req.query.is_empty() {
        return path;
    }
    for (i, (key, value)) in req.query.iter().enumerate() {
        path.push(if i == 0 { '?' } else { '&' });
        path.push_str(&pct_encode(key));
        path.push('=');
        path.push_str(&pct_encode(value));
    }
    path
}

fn pct_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn cpu_usage(sys: &System) -> Vec<f64> {
    // sysinfo can transiently report slightly over 100% per core between
    // refreshes; clamp so the artifact stays inside its declared range.
    let mut out: Vec<f64> = sys
        .cpus()
        .iter()
        .map(|c| f64::from(c.cpu_usage()).clamp(0.0, 100.0))
        .collect();
    if out.is_empty() {
        out.push(0.0);
    }
    out
}

fn process_tree_memory_mb(sys: &mut System, root: Pid) -> Option<f64> {
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_memory(),
    );
    sys.process(root)?;

    let mut total = 0_u64;
    for (&pid, process) in sys.processes() {
        if pid == root || is_descendant_process(sys, pid, root) {
            total = total.saturating_add(process.memory());
        }
    }
    Some(total as f64 / (1024.0 * 1024.0))
}

fn is_descendant_process(sys: &System, pid: Pid, root: Pid) -> bool {
    let mut current = pid;
    for _ in 0..64 {
        let Some(parent) = sys.process(current).and_then(|process| process.parent()) else {
            return false;
        };
        if parent == root {
            return true;
        }
        current = parent;
    }
    false
}

fn summarize_latency(values: &[f64]) -> Latency {
    // Sort once: `percentile` would otherwise clone and sort the whole sample
    // set five times, which is measurable at trial-aggregate sizes.
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Latency {
        avg: avg(values),
        p50: Some(percentile_sorted(&sorted, 0.50)),
        p90: Some(percentile_sorted(&sorted, 0.90)),
        p95: percentile_sorted(&sorted, 0.95),
        p99: percentile_sorted(&sorted, 0.99),
        p999: Some(percentile_sorted(&sorted, 0.999)),
    }
}

fn resolve_path(value: Option<PathBuf>, key: &str, flag: &str) -> Result<PathBuf, Fail> {
    value
        .or_else(|| std::env::var_os(key).map(PathBuf::from))
        .ok_or_else(|| Fail::new(Code::InvalidCli, format!("missing {flag} or {key}")))
}

pub(crate) fn resolve_text(value: Option<String>, key: &str, flag: &str) -> Result<String, Fail> {
    value
        .or_else(|| std::env::var(key).ok())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Fail::new(Code::InvalidCli, format!("missing {flag} or {key}")))
}

pub(crate) fn resolve_num<T: std::str::FromStr>(
    value: Option<T>,
    key: &str,
    flag: &str,
) -> Result<T, Fail> {
    if let Some(v) = value {
        return Ok(v);
    }
    std::env::var(key)
        .map_err(|_| Fail::new(Code::InvalidCli, format!("missing {flag} or {key}")))?
        .parse::<T>()
        .map_err(|_| Fail::new(Code::InvalidCli, format!("invalid {flag} or {key}")))
}

fn resolve_suite(value: Option<Suite>, key: &str) -> Result<Suite, Fail> {
    if let Some(v) = value {
        return Ok(v);
    }
    let raw =
        std::env::var(key).map_err(|_| Fail::new(Code::InvalidCli, format!("missing {key}")))?;
    match raw.as_str() {
        "throughput-http" => Ok(Suite::ThroughputHttp),
        _ => Err(Fail::new(Code::InvalidCli, format!("invalid {key}"))),
    }
}

fn pg_url() -> String {
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

// ---------------------------------------------------------------------------
// Shared response / query-param types
// ---------------------------------------------------------------------------

/// Query parameters accepted by every benchmark route.
///
/// `seed` and `idx` are stamped onto every generated request so the request
/// stream is reproducible and self-describing; handlers ignore them, but they
/// must still be accepted rather than rejected as unknown.
#[derive(Debug, Deserialize)]
pub(crate) struct QueryParams {
    pub(crate) id: Option<i32>,
    pub(crate) limit: Option<usize>,
    pub(crate) offset: Option<usize>,
    pub(crate) term: Option<String>,
    #[serde(default, rename = "seed")]
    _seed: Option<u64>,
    #[serde(default, rename = "idx")]
    _idx: Option<u64>,
}

impl QueryParams {
    pub(crate) fn limit_or(&self, default: usize) -> usize {
        self.limit.unwrap_or(default)
    }

    pub(crate) fn offset(&self) -> usize {
        self.offset.unwrap_or(0)
    }

    pub(crate) fn user_id(&self, n: i32) -> i32 {
        self.id.map(|i| (i - 1).rem_euclid(n) + 1).unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn request_plan_groups_query_metrics_by_method_and_route() {
        let requests = vec![
            request("/customers", &[("id", "1")]),
            request("/customers", &[("id", "2")]),
            request("/products", &[("id", "1")]),
        ];

        let (plans, keys) = request_plan(&requests);

        assert_eq!(plans.len(), 3);
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].path, "/customers");
        assert_eq!(keys[1].path, "/products");
        assert_eq!(plans[0].query_idx, plans[1].query_idx);
        assert_ne!(plans[0].query_idx, plans[2].query_idx);
        assert!(plans[0].path.contains("id=1"));
    }

    #[test]
    fn query_params_user_id_maps_to_inclusive_seed_range() {
        assert_eq!(query_params_id(1).user_id(10), 1);
        assert_eq!(query_params_id(10).user_id(10), 10);
        assert_eq!(query_params_id(11).user_id(10), 1);
    }

    #[test]
    fn query_points_report_route_level_metrics() {
        let keys = vec![QueryKey {
            method: "GET".to_string(),
            path: "/customers".to_string(),
        }];
        let mut buckets = vec![QueryBucket::default()];
        buckets[0].record(true, 10.0);
        buckets[0].record(true, 20.0);
        buckets[0].record(false, 0.0);

        let points = query_points(&keys, &buckets, 2.0);

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].path, "/customers");
        assert_eq!(points[0].rps, 1.5);
        assert_eq!(points[0].err, 1.0 / 3.0);
        assert_eq!(points[0].latency.avg, 15.0);
    }

    fn stage(sec: u32, vus: u32) -> Stage {
        Stage {
            sec,
            vus: Some(vus),
            rps: None,
        }
    }

    fn vus_of(schedule: &[SecondPlan]) -> Vec<u32> {
        schedule.iter().map(|slot| slot.vus).collect()
    }

    fn phases_of(schedule: &[SecondPlan]) -> Vec<Phase> {
        schedule.iter().map(|slot| slot.phase).collect()
    }

    #[test]
    fn ramping_schedule_approximates_k6_ramp_without_empty_first_bucket() {
        let schedule = build_schedule(&[stage(5, 200)], true, 0);

        assert_eq!(vus_of(&schedule), vec![40, 80, 120, 160, 200]);
        assert_eq!(phases_of(&schedule), vec![Phase::Ramp; 5]);
    }

    #[test]
    fn ramping_schedule_holds_once_the_target_stops_moving() {
        let schedule = build_schedule(&[stage(2, 200), stage(3, 200)], true, 0);

        assert_eq!(vus_of(&schedule), vec![100, 200, 200, 200, 200]);
        assert_eq!(
            phases_of(&schedule),
            vec![
                Phase::Ramp,
                Phase::Ramp,
                Phase::Hold,
                Phase::Hold,
                Phase::Hold
            ]
        );
        assert_eq!(
            schedule.iter().map(|slot| slot.stage).collect::<Vec<_>>(),
            vec![0, 0, 1, 1, 1]
        );
    }

    #[test]
    fn constant_vus_schedule_holds_at_the_declared_count() {
        // constant-vus means constant. Interpolating from zero would spend the
        // whole stage below the requested concurrency and report the average of
        // a ramp as if it were a steady-state measurement.
        let schedule = build_schedule(&[stage(5, 200)], false, 0);

        assert_eq!(vus_of(&schedule), vec![200; 5]);
        assert_eq!(phases_of(&schedule), vec![Phase::Hold; 5]);
    }

    #[test]
    fn warmup_seconds_are_tagged_ahead_of_everything_else() {
        let schedule = build_schedule(&[stage(4, 100)], false, 2);

        assert_eq!(
            phases_of(&schedule),
            vec![Phase::Warmup, Phase::Warmup, Phase::Hold, Phase::Hold]
        );
    }

    #[test]
    fn trial_rotation_is_deterministic_and_trial_specific() {
        let requests = (0..512)
            .map(|i| request("/customers", &[("id", &i.to_string())]))
            .collect::<Vec<_>>();
        let (base, _) = request_plan(&requests);
        let paths = |plans: &[RequestPlan]| {
            plans
                .iter()
                .map(|plan| plan.path.clone())
                .collect::<Vec<_>>()
        };

        let rotated = |trial: u32| {
            let mut plans = base.clone();
            rotate_for_trial(&mut plans, 42, trial);
            plans
        };

        assert_eq!(
            paths(&rotated(1)),
            paths(&rotated(1)),
            "same seed+trial must be reproducible"
        );
        assert_eq!(rotated(1).len(), base.len(), "rotation must not drop plans");

        // Five trials must not all replay the same request order.
        let orders = (0..5)
            .map(|trial| paths(&rotated(trial)))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(orders.len(), 5, "each trial must get its own request order");
    }

    #[test]
    fn status_line_parsing_accepts_any_http1x_2xx() {
        assert_eq!(parse_status_code("HTTP/1.1 200 OK"), Some(200));
        assert_eq!(parse_status_code("HTTP/1.0 200 OK"), Some(200));
        assert_eq!(parse_status_code("HTTP/1.1 201 Created"), Some(201));
        assert_eq!(parse_status_code("HTTP/1.1 404 Not Found"), Some(404));
        assert_eq!(parse_status_code("garbage"), None);
    }

    #[test]
    fn summarize_latency_reports_measured_p50_and_p90() {
        // Nearest-rank over 0..=100, so the q-th percentile is exactly q.
        let values: Vec<f64> = (0..=100).map(f64::from).collect();
        let latency = summarize_latency(&values);

        assert_eq!(latency.p50, Some(50.0));
        assert_eq!(latency.p90, Some(90.0));
        assert_eq!(latency.p95, 95.0);
        assert_eq!(latency.p99, 99.0);
    }

    fn request(path: &str, query: &[(&str, &str)]) -> RequestDoc {
        RequestDoc {
            method: "GET".to_string(),
            path: path.to_string(),
            query: query
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    fn query_params_id(id: i32) -> QueryParams {
        QueryParams {
            id: Some(id),
            limit: None,
            offset: None,
            term: None,
            _seed: None,
            _idx: None,
        }
    }
}
