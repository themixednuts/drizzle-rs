mod pg_sync;
mod pg_tokio;
mod sqlite;
mod turso;

use crate::cli::{Load, Suite};
use crate::code::{Code, Fail};
use crate::model::{Latency, Point, RequestDoc, Workload};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use sysinfo::System;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::sync::oneshot;

/// Start the adapter server for the given target. Used by both `load` and `parity`.
pub(crate) async fn serve_target(
    target: &str,
    seed: u64,
    trial: u32,
) -> Result<ServerHandle, Fail> {
    match target {
        "drizzle-rs-sqlite" => sqlite::serve(seed, trial).await,
        "drizzle-rs-pg-sync" => pg_sync::serve(seed, trial).await,
        "drizzle-rs-pg-tokio" => pg_tokio::serve(seed, trial).await,
        "drizzle-rs-turso" => turso::serve(seed, trial).await,
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
    let trial = resolve_num(args.trial, "BENCH_TRIAL", "--trial")?;
    let seed = resolve_num(args.seed, "BENCH_SEED", "--seed")?;
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

    let handle = serve_target(&target, seed, trial).await?;

    let port = handle.port;
    let points = tokio::task::spawn_blocking(move || measure(&workload, &requests, port))
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
}

impl ServerHandle {
    pub(crate) async fn shutdown(mut self) -> Result<(), Fail> {
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
    })
}

// ---------------------------------------------------------------------------
// Measurement loop (sync, runs in spawn_blocking)
// ---------------------------------------------------------------------------

fn measure(workload: &Workload, requests: &[RequestDoc], port: u16) -> Result<Vec<Point>, Fail> {
    let total_s: u32 = workload.stages.iter().map(|s| s.sec).sum();
    let bucket_s = workload.sampling.bucket_s.max(1);
    let mut remaining = total_s;
    let mut cursor = 0_usize;
    let mut points = Vec::new();
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();

    while remaining > 0 {
        let sec = remaining.min(bucket_s);
        let window = Duration::from_secs(sec as u64);
        let start = Instant::now();
        let deadline = start + window;
        let mut latencies = Vec::new();
        let mut errors = 0_u64;
        let mut total = 0_u64;
        let mut first_err = None;

        while Instant::now() < deadline {
            let item = &requests[cursor % requests.len()];
            cursor += 1;
            let t0 = Instant::now();
            match send_request(port, item) {
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

        if total > 0 && errors == total {
            return Err(Fail::new(
                Code::RunFail,
                first_err.unwrap_or_else(|| "all requests failed".to_string()),
            ));
        }

        sys.refresh_cpu_usage();
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
        });
        remaining -= sec;
    }

    Ok(points)
}

fn send_request(port: u16, req: &RequestDoc) -> Result<(), String> {
    send_get(port, &build_path(req))
}

fn send_get(port: u16, path: &str) -> Result<(), String> {
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
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|err| format!("read failed: {err}"))?;

    if status_line.starts_with("HTTP/1.1 200") || status_line.starts_with("HTTP/1.1 204") {
        Ok(())
    } else {
        Err(format!("request failed: {}", status_line.trim()))
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
    id: Option<i32>,
    idx: Option<usize>,
    #[allow(dead_code)]
    q: Option<String>,
    #[allow(dead_code)]
    seed: Option<u64>,
}

impl QueryParams {
    pub(crate) fn page(&self) -> usize {
        self.idx.map(|i| i % 64).unwrap_or(0)
    }

    pub(crate) fn user_id(&self, n: i32) -> i32 {
        self.id.map(|i| i.rem_euclid(n).max(1)).unwrap_or(1)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct UserRow {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct PostRow {
    pub id: i32,
    pub title: String,
    pub author_id: i32,
}

#[derive(Debug, Serialize)]
pub(crate) struct DetailRow {
    pub name: String,
    pub title: String,
}
