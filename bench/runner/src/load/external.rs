use super::*;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;

/// How long to wait for a target to announce its port before giving up.
///
/// A child that exits is detected immediately via EOF regardless of this value;
/// the timeout only bounds a child that stays alive without announcing. The
/// default is generous because `cargo run --release` server commands may pay a
/// cold build before the server starts (CI prebuilds; local first runs do not).
/// Override with `BENCH_LISTENING_TIMEOUT_S`.
const LISTENING_TIMEOUT_DEFAULT_S: u64 = 600;

fn listening_timeout() -> Duration {
    let secs = std::env::var("BENCH_LISTENING_TIMEOUT_S")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(LISTENING_TIMEOUT_DEFAULT_S);
    Duration::from_secs(secs)
}

/// Spawn an external HTTP server process and wait for it to print its port.
///
/// The process is expected to print `LISTENING port=<N>` to stdout once ready.
///
/// Both pipes are drained by background threads for the entire lifetime of the
/// child. Stopping at the `LISTENING` line leaves the pipe buffer to fill, and
/// the next `console.log` in a Bun target then dies on EPIPE mid-benchmark.
///
/// The child is placed in its own process group so shutdown can kill the whole
/// subtree — `bun run`, `cargo run` and shell wrappers all leave the real server
/// running if only the direct child is signalled.
pub async fn serve(cmd_json: &str, target: &str, seed: u64) -> Result<(ServerHandle, Child), Fail> {
    let cmd: Vec<String> = serde_json::from_str(cmd_json).map_err(|e| {
        Fail::new(
            Code::InvalidCli,
            format!("invalid BENCH_SERVER_CMD JSON: {e}"),
        )
    })?;
    if cmd.is_empty() {
        return Err(Fail::new(
            Code::InvalidCli,
            "BENCH_SERVER_CMD is empty".to_string(),
        ));
    }

    let cwd = std::env::var("BENCH_SERVER_CWD").ok();

    let resolved_cmd = resolve_cmd_token(&cmd[0])?;
    let resolved_args = cmd[1..]
        .iter()
        .map(|arg| resolve_arg_token(arg))
        .collect::<Result<Vec<_>, _>>()?;

    let mut builder = Command::new(&resolved_cmd);
    builder.args(&resolved_args);
    if let Some(dir) = &cwd {
        builder.current_dir(dir);
    }
    // Forward run identity so external targets can load the shared seed data.
    builder.env("BENCH_TARGET_ID", target);
    builder.env("BENCH_SEED", seed.to_string());
    for key in [
        "BENCH_SEED_FILE",
        "BENCH_TRIAL",
        "BENCH_POOL_SIZE",
        "BENCH_WORKERS",
    ] {
        if let Ok(value) = std::env::var(key) {
            builder.env(key, value);
        }
    }
    if let Ok(current_exe) = std::env::current_exe() {
        builder.env("BENCH_RUNNER_BIN", current_exe);
    }
    builder
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    crate::proc::spawn_server_child(&mut builder);

    let mut child = builder.spawn().map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("failed to spawn external server {cmd:?}: {e}"),
        )
    })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Fail::new(Code::RunFail, "external server has no stdout".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Fail::new(Code::RunFail, "external server has no stderr".to_string()))?;

    let (port_tx, port_rx) = mpsc::channel::<Result<u16, String>>();
    std::thread::Builder::new()
        .name("bench-target-stdout".to_string())
        .spawn(move || {
            let reader = BufReader::new(stdout);
            let mut announced = false;
            for line in reader.lines().map_while(Result::ok) {
                if !announced && let Some(rest) = line.strip_prefix("LISTENING port=") {
                    announced = true;
                    let _ = port_tx.send(
                        rest.trim()
                            .parse::<u16>()
                            .map_err(|e| format!("invalid port in LISTENING line: {e}")),
                    );
                    continue;
                }
                // Keep reading for the rest of the process lifetime. The content
                // is not interesting, but an unread pipe is fatal to the target.
            }
            if !announced {
                let _ = port_tx.send(Err(
                    "external server exited without printing LISTENING port=<N>".to_string(),
                ));
            }
        })
        .map_err(|e| Fail::new(Code::RunFail, format!("stdout drain thread failed: {e}")))?;

    std::thread::Builder::new()
        .name("bench-target-stderr".to_string())
        .spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                eprintln!("[target] {line}");
            }
        })
        .map_err(|e| Fail::new(Code::RunFail, format!("stderr drain thread failed: {e}")))?;

    let port = tokio::task::spawn_blocking(move || {
        port_rx
            .recv_timeout(listening_timeout())
            .map_err(|_| "timed out waiting for LISTENING port=<N>".to_string())?
    })
    .await
    .map_err(|e| Fail::new(Code::RunFail, format!("port reader panicked: {e}")))?;

    let port = match port {
        Ok(port) => port,
        Err(msg) => {
            crate::proc::kill_tree(&mut child);
            return Err(Fail::new(Code::TargetFail, msg));
        }
    };

    Ok((
        ServerHandle {
            port,
            stop: None,
            task: None,
            workers: Vec::new(),
            temp_dirs: Vec::new(),
            external_child: None,
            target_pid: None,
        },
        child,
    ))
}

pub(crate) fn resolve_cmd_token(token: &str) -> Result<String, Fail> {
    if token == "$BENCH_RUNNER_BIN" {
        return std::env::current_exe()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|err| {
                Fail::new(
                    Code::RunFail,
                    format!("failed to resolve BENCH_RUNNER_BIN: {err}"),
                )
            });
    }
    resolve_arg_token(token)
}

pub(crate) fn resolve_arg_token(token: &str) -> Result<String, Fail> {
    if let Some(name) = token.strip_prefix('$') {
        return std::env::var(name).map_err(|err| {
            Fail::new(
                Code::InvalidCli,
                format!("failed to expand server command token {token}: {err}"),
            )
        });
    }
    Ok(token.to_string())
}
