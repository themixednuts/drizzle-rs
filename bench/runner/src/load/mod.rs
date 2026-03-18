mod external;
mod pg_sync;
mod pg_tokio;
mod spacetime_pg;
mod sqlite;
mod turso;

use crate::cli::{Load, Suite};
use crate::code::{Code, Fail};
use crate::model::{Latency, Point, RequestDoc, Workload};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read as _, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::sync::oneshot;

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
// Details per order: controlled via SeedConfig::relation() (~6 per order, matching upstream avg)

/// Start the adapter server for the given target. Used by both `load` and `parity`.
///
/// Built-in targets dispatch to compiled Rust implementations.
/// External targets (indicated by `BENCH_SERVER_CMD` env var) are spawned as
/// child processes that must print `LISTENING port=<N>` to stdout.
pub(crate) async fn serve_target(target: &str, seed: u64) -> Result<ServerHandle, Fail> {
    // Check for external server command first
    if let Ok(cmd_json) = std::env::var("BENCH_SERVER_CMD") {
        let (mut handle, child) = external::serve(&cmd_json).await?;
        handle.target_pid = Some(child.id());
        handle.external_child = Some(child);
        return Ok(handle);
    }

    match target {
        "drizzle-rs-sqlite" => sqlite::serve(seed).await,
        "drizzle-rs-pg-sync" => pg_sync::serve(seed).await,
        "drizzle-rs-pg-tokio" => pg_tokio::serve(seed).await,
        "spacetime-pgwire-rs" => spacetime_pg::serve(seed).await,
        "drizzle-rs-turso" => turso::serve(seed).await,
        other => Err(Fail::new(
            Code::InvalidCli,
            format!("unsupported target: {other}"),
        )),
    }
}

/// Send an HTTP GET and return the full response body (or error).
pub(crate) fn send_get_body(port: u16, path: &str) -> Result<(u16, String), String> {
    let mut stream =
        TcpStream::connect(("127.0.0.1", port)).map_err(|err| format!("connect failed: {err}"))?;
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

    let mut reader = BufReader::new(stream);

    // Parse status line
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|err| format!("read status failed: {err}"))?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    // Skip headers until blank line
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|err| format!("read header failed: {err}"))?;
        if let Some(val) = line
            .strip_prefix("Content-Length: ")
            .or_else(|| line.strip_prefix("content-length: "))
        {
            content_length = val.trim().parse().ok();
        }
        if line.trim().is_empty() {
            break;
        }
    }

    // Read body
    let mut body = String::new();
    if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        std::io::Read::read_exact(&mut reader, &mut buf)
            .map_err(|err| format!("read body failed: {err}"))?;
        body = String::from_utf8_lossy(&buf).into_owned();
    } else {
        std::io::Read::read_to_string(&mut reader, &mut body)
            .map_err(|err| format!("read body failed: {err}"))?;
    }

    Ok((status_code, body))
}

pub async fn run(args: Load) -> Result<Code, Fail> {
    let out = resolve_path(args.out, "BENCH_TIMESERIES_OUT", "--out")?;
    let target = resolve_text(args.target, "BENCH_TARGET_ID", "--target")?;
    let _trial: u32 = resolve_num(args.trial, "BENCH_TRIAL", "--trial")?;
    let seed: u64 = resolve_num(args.seed, "BENCH_SEED", "--seed")?;
    let suite = resolve_suite(args.suite, "BENCH_SUITE")?;
    let workload_path = resolve_path(args.workload, "BENCH_WORKLOAD_FILE", "--workload")?;
    let requests_path = resolve_path(args.requests, "BENCH_REQUESTS_FILE", "--requests")?;

    let workload = load_json::<Workload>(&workload_path)?;
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
    let requests = load_json::<Vec<RequestDoc>>(&requests_path)?;
    if requests.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            "load requires a non-empty requests file",
        ));
    }

    let handle = serve_target(&target, seed).await?;

    let port = handle.port;
    let target_pid = handle.target_pid;
    let points =
        tokio::task::spawn_blocking(move || measure(&workload, &requests, port, target_pid))
            .await
            .map_err(|err| Fail::new(Code::RunFail, format!("measure panicked: {err}")))??;

    handle.shutdown().await?;
    write_json(out, &points)?;
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
    pub(crate) external_child: Option<std::process::Child>,
    pub(crate) target_pid: Option<u32>,
}

impl ServerHandle {
    pub(crate) async fn shutdown(mut self) -> Result<(), Fail> {
        // Kill external child process first
        if let Some(mut child) = self.external_child.take() {
            let _ = child.kill();
            let _ = child.wait();
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
        Ok(())
    }
}

/// Bind an ephemeral port and spawn `app` as a tokio task.
async fn spawn_server(app: Router) -> Result<ServerHandle, Fail> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|err| Fail::new(Code::RunFail, format!("bind failed: {err}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| Fail::new(Code::RunFail, format!("set_nonblocking failed: {err}")))?;
    let port = listener
        .local_addr()
        .map_err(|err| Fail::new(Code::RunFail, format!("local_addr failed: {err}")))?
        .port();

    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .map_err(|err| format!("server init failed: {err}"))?
            .serve(app.into_make_service())
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
        external_child: None,
        target_pid: Some(std::process::id()),
    })
}

// ---------------------------------------------------------------------------
// Measurement loop — concurrent workers, each with a keep-alive connection
// ---------------------------------------------------------------------------

/// Per-worker results for one sampling bucket.
struct BucketResult {
    latencies: Vec<f64>,
    errors: u64,
    total: u64,
    first_err: Option<String>,
}

fn measure(
    workload: &Workload,
    requests: &[RequestDoc],
    port: u16,
    target_pid: Option<u32>,
) -> Result<Vec<Point>, Fail> {
    if workload.load.executor.contains("vus") {
        measure_vus(workload, requests, port, target_pid)
    } else {
        measure_rps(workload, requests, port, target_pid)
    }
}

fn measure_rps(
    workload: &Workload,
    requests: &[RequestDoc],
    port: u16,
    target_pid: Option<u32>,
) -> Result<Vec<Point>, Fail> {
    let concurrency = workload.load.concurrency.max(1) as usize;
    let total_s: u32 = workload.stages.iter().map(|s| s.sec).sum();
    let bucket_s = workload.sampling.bucket_s.max(1);
    let mut remaining = total_s;
    let mut points = Vec::new();
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    let pid = target_pid.map(Pid::from_u32);

    // Pre-compute request paths once (avoid allocs in hot loop).
    let paths: Arc<Vec<String>> = Arc::new(requests.iter().map(build_path).collect());

    while remaining > 0 {
        let sec = remaining.min(bucket_s);
        let window = Duration::from_secs(sec as u64);
        let deadline = Instant::now() + window;
        let start = Instant::now();

        // Spawn workers that fire requests until the deadline.
        let handles: Vec<_> = (0..concurrency)
            .map(|worker_id| {
                let paths = Arc::clone(&paths);
                std::thread::spawn(move || {
                    let mut conn = HttpConn::new(port);
                    let mut latencies = Vec::with_capacity(1024);
                    let mut errors = 0_u64;
                    let mut total = 0_u64;
                    let mut first_err = None;
                    // Each worker starts at a different offset to spread requests.
                    let mut cursor = worker_id;

                    while Instant::now() < deadline {
                        let path = &paths[cursor % paths.len()];
                        cursor += concurrency;
                        let t0 = Instant::now();
                        match conn.get(path) {
                            Ok(()) => latencies.push(t0.elapsed().as_secs_f64() * 1000.0),
                            Err(msg) => {
                                errors += 1;
                                if first_err.is_none() {
                                    first_err = Some(msg);
                                }
                            }
                        }
                        total += 1;
                    }

                    BucketResult {
                        latencies,
                        errors,
                        total,
                        first_err,
                    }
                })
            })
            .collect();

        // Merge worker results.
        let mut latencies = Vec::new();
        let mut errors = 0_u64;
        let mut total = 0_u64;
        let mut first_err = None;
        for handle in handles {
            let result = handle
                .join()
                .map_err(|_| Fail::new(Code::RunFail, "worker thread panicked".to_string()))?;
            latencies.extend(result.latencies);
            errors += result.errors;
            total += result.total;
            if first_err.is_none() {
                first_err = result.first_err;
            }
        }

        if total > 0 && errors == total {
            return Err(Fail::new(
                Code::RunFail,
                first_err.unwrap_or_else(|| "all requests failed".to_string()),
            ));
        }

        sys.refresh_cpu_usage();
        let mem_mb = pid.and_then(|p| {
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[p]),
                true,
                ProcessRefreshKind::nothing().with_memory(),
            );
            sys.process(p)
                .map(|proc| proc.memory() as f64 / (1024.0 * 1024.0))
        });
        let wall = start.elapsed().as_secs_f64().max(0.001);
        points.push(Point {
            time: now_rfc3339(),
            rps: total as f64 / wall,
            err: if total == 0 {
                0.0
            } else {
                errors as f64 / total as f64
            },
            latency: summarize_latency(&latencies),
            cpu: cpu_usage(&sys),
            mem_mb,
        });
        remaining -= sec;
    }

    Ok(points)
}

fn measure_vus(
    workload: &Workload,
    requests: &[RequestDoc],
    port: u16,
    target_pid: Option<u32>,
) -> Result<Vec<Point>, Fail> {
    let schedule = build_vu_schedule(&workload.stages);
    if schedule.is_empty() {
        return Err(Fail::new(Code::RunFail, "empty VU schedule"));
    }
    let bucket_s = workload.sampling.bucket_s.max(1) as usize;
    let paths: Arc<Vec<String>> = Arc::new(requests.iter().map(build_path).collect());
    let global_iter = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let (tx, rx) = mpsc::channel::<(bool, f64)>();

    let mut workers: Vec<std::thread::JoinHandle<()>> = Vec::new();
    let mut points = Vec::new();
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    let pid = target_pid.map(Pid::from_u32);

    let mut sec_offset = 0;
    while sec_offset < schedule.len() {
        let chunk_end = (sec_offset + bucket_s).min(schedule.len());
        let target_vus = *schedule[sec_offset..chunk_end].iter().max().unwrap_or(&0) as usize;

        // Grow worker pool as VU count increases (ramping-vus only ramps up)
        while workers.len() < target_vus {
            let paths = Arc::clone(&paths);
            let iter_counter = Arc::clone(&global_iter);
            let is_running = Arc::clone(&running);
            let sender = tx.clone();
            workers.push(std::thread::spawn(move || {
                let mut conn = HttpConn::new(port);
                while is_running.load(Ordering::Relaxed) {
                    let iter_num = iter_counter.fetch_add(1, Ordering::Relaxed);
                    let path = &paths[iter_num as usize % paths.len()];

                    let t0 = Instant::now();
                    let ok = conn.get(path).is_ok();
                    let lat = t0.elapsed().as_secs_f64() * 1000.0;

                    if sender.send((ok, lat)).is_err() {
                        break;
                    }

                    // k6-compatible sleep: sleep(0.1 * (iteration % 6))
                    let sleep_ms = (iter_num % 6) * 100;
                    if sleep_ms > 0 {
                        std::thread::sleep(Duration::from_millis(sleep_ms));
                    }
                }
            }));
        }

        let chunk_secs = (chunk_end - sec_offset) as u64;
        let start = Instant::now();
        std::thread::sleep(Duration::from_secs(chunk_secs));
        let wall = start.elapsed().as_secs_f64().max(0.001);

        // Drain results collected during this window
        let mut latencies = Vec::new();
        let mut errors = 0u64;
        let mut total = 0u64;
        while let Ok((ok, lat)) = rx.try_recv() {
            total += 1;
            if ok {
                latencies.push(lat);
            } else {
                errors += 1;
            }
        }

        sys.refresh_cpu_usage();
        let mem_mb = pid.and_then(|p| {
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[p]),
                true,
                ProcessRefreshKind::nothing().with_memory(),
            );
            sys.process(p)
                .map(|proc| proc.memory() as f64 / (1024.0 * 1024.0))
        });
        points.push(Point {
            time: now_rfc3339(),
            rps: total as f64 / wall,
            err: if total == 0 {
                0.0
            } else {
                errors as f64 / total as f64
            },
            latency: summarize_latency(&latencies),
            cpu: cpu_usage(&sys),
            mem_mb,
        });

        sec_offset = chunk_end;
    }

    running.store(false, Ordering::Relaxed);
    drop(tx);
    for h in workers {
        let _ = h.join();
    }

    Ok(points)
}

/// Build a per-second VU schedule from stages, linearly interpolating ramp stages.
fn build_vu_schedule(stages: &[crate::model::Stage]) -> Vec<u32> {
    let mut schedule = Vec::new();
    let mut prev = 0u32;
    for stage in stages {
        let target = stage.vus.unwrap_or(prev);
        if stage.sec == 0 {
            continue;
        }
        if stage.sec == 1 {
            schedule.push(target);
        } else {
            for i in 0..stage.sec {
                let t = i as f64 / (stage.sec - 1) as f64;
                let vus = prev as f64 + (target as f64 - prev as f64) * t;
                schedule.push(vus.round() as u32);
            }
        }
        prev = target;
    }
    schedule
}

/// Persistent HTTP/1.1 connection with keep-alive. Reconnects on error.
struct HttpConn {
    port: u16,
    reader: Option<BufReader<TcpStream>>,
    header_buf: String,
}

impl HttpConn {
    fn new(port: u16) -> Self {
        Self {
            port,
            reader: None,
            header_buf: String::new(),
        }
    }

    fn connect(&mut self) -> Result<(), String> {
        let stream = TcpStream::connect(("127.0.0.1", self.port))
            .map_err(|err| format!("connect failed: {err}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|err| format!("set_read_timeout: {err}"))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|err| format!("set_write_timeout: {err}"))?;
        stream
            .set_nodelay(true)
            .map_err(|err| format!("set_nodelay: {err}"))?;
        self.reader = Some(BufReader::new(stream));
        Ok(())
    }

    fn get(&mut self, path: &str) -> Result<(), String> {
        // Try on existing connection first, reconnect once on failure.
        if self.reader.is_some() {
            match self.send_and_read(path) {
                Ok(()) => return Ok(()),
                Err(_) => self.reader = None,
            }
        }
        self.connect()?;
        self.send_and_read(path)
    }

    fn send_and_read(&mut self, path: &str) -> Result<(), String> {
        let reader = self.reader.as_mut().ok_or("no connection")?;
        let raw =
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: keep-alive\r\n\r\n");
        reader
            .get_mut()
            .write_all(raw.as_bytes())
            .map_err(|err| format!("write failed: {err}"))?;

        // Read status line
        self.header_buf.clear();
        reader
            .read_line(&mut self.header_buf)
            .map_err(|err| format!("read status: {err}"))?;
        let ok = self.header_buf.starts_with("HTTP/1.1 200")
            || self.header_buf.starts_with("HTTP/1.1 204");

        // Read headers, extract Content-Length or detect chunked encoding
        let mut content_length: Option<usize> = None;
        let mut chunked = false;
        loop {
            self.header_buf.clear();
            reader
                .read_line(&mut self.header_buf)
                .map_err(|err| format!("read header: {err}"))?;
            let line = self.header_buf.trim();
            if line.is_empty() {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if let Some(val) = lower.strip_prefix("content-length:") {
                content_length = val.trim().parse().ok();
            } else if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
                chunked = true;
            }
        }

        // Drain body so the connection is ready for the next request
        if let Some(len) = content_length {
            let mut remaining = len;
            let mut discard = [0u8; 8192];
            while remaining > 0 {
                let to_read = remaining.min(discard.len());
                let n = reader
                    .read(&mut discard[..to_read])
                    .map_err(|err| format!("read body: {err}"))?;
                if n == 0 {
                    break;
                }
                remaining -= n;
            }
        } else if chunked {
            // Read chunked transfer encoding
            loop {
                self.header_buf.clear();
                reader
                    .read_line(&mut self.header_buf)
                    .map_err(|err| format!("read chunk size: {err}"))?;
                let size = usize::from_str_radix(self.header_buf.trim(), 16).unwrap_or(0);
                if size == 0 {
                    // Read trailing \r\n after last chunk
                    self.header_buf.clear();
                    let _ = reader.read_line(&mut self.header_buf);
                    break;
                }
                let mut remaining = size;
                let mut discard = [0u8; 8192];
                while remaining > 0 {
                    let to_read = remaining.min(discard.len());
                    let n = reader
                        .read(&mut discard[..to_read])
                        .map_err(|err| format!("read chunk: {err}"))?;
                    if n == 0 {
                        break;
                    }
                    remaining -= n;
                }
                // Read trailing \r\n after chunk data
                self.header_buf.clear();
                let _ = reader.read_line(&mut self.header_buf);
            }
        }

        if ok {
            Ok(())
        } else {
            Err(format!("request failed: {}", self.header_buf.trim()))
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

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
    let mut out: Vec<f64> = sys
        .cpus()
        .iter()
        .map(|c| f64::from(c.cpu_usage()))
        .collect();
    if out.is_empty() {
        out.push(0.0);
    }
    out
}

fn summarize_latency(values: &[f64]) -> Latency {
    Latency {
        avg: avg(values),
        p95: percentile(values, 0.95),
        p99: percentile(values, 0.99),
        p999: Some(percentile(values, 0.999)),
    }
}

fn avg(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn percentile(values: &[f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Fail> {
    let body = fs::read_to_string(path).map_err(|err| {
        Fail::new(
            Code::InvalidInput,
            format!("failed to read {}: {err}", path.display()),
        )
    })?;
    serde_json::from_str(&body).map_err(|err| {
        Fail::new(
            Code::InvalidInput,
            format!("invalid json {}: {err}", path.display()),
        )
    })
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) -> Result<(), Fail> {
    let body = serde_json::to_string_pretty(value).map_err(|err| {
        Fail::new(
            Code::RunFail,
            format!("serialize {} failed: {err}", path.display()),
        )
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            Fail::new(
                Code::RunFail,
                format!("failed to create {}: {err}", parent.display()),
            )
        })?;
    }
    fs::write(&path, body).map_err(|err| {
        Fail::new(
            Code::RunFail,
            format!("write {} failed: {err}", path.display()),
        )
    })
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
        "mvcc-contention" => Ok(Suite::MvccContention),
        _ => Err(Fail::new(Code::InvalidCli, format!("invalid {key}"))),
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
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

#[derive(Debug, Deserialize)]
pub(crate) struct QueryParams {
    pub(crate) id: Option<i32>,
    pub(crate) limit: Option<usize>,
    pub(crate) offset: Option<usize>,
    #[allow(dead_code)]
    pub(crate) q: Option<String>,
    #[allow(dead_code)]
    pub(crate) seed: Option<u64>,
    pub(crate) term: Option<String>,
}

impl QueryParams {
    pub(crate) fn limit_or(&self, default: usize) -> usize {
        self.limit.unwrap_or(default)
    }

    pub(crate) fn offset(&self) -> usize {
        self.offset.unwrap_or(0)
    }

    pub(crate) fn user_id(&self, n: i32) -> i32 {
        self.id.map(|i| i.rem_euclid(n).max(1)).unwrap_or(1)
    }
}
