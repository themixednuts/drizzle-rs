use super::*;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

/// Spawn an external HTTP server process and wait for it to print its port.
///
/// The process is expected to print `LISTENING port=<N>` to stdout once ready.
/// All other stdout output is discarded. stderr is inherited.
///
/// The seed data file path is forwarded via `BENCH_SEED_FILE` env var (inherited
/// from the parent process). External targets read that file for seeding.
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
    if let Ok(seed_file) = std::env::var("BENCH_SEED_FILE") {
        builder.env("BENCH_SEED_FILE", seed_file);
    }
    if let Ok(trial) = std::env::var("BENCH_TRIAL") {
        builder.env("BENCH_TRIAL", trial);
    }
    if let Ok(pool_size) = std::env::var("BENCH_POOL_SIZE") {
        builder.env("BENCH_POOL_SIZE", pool_size);
    }
    if let Ok(current_exe) = std::env::current_exe() {
        builder.env("BENCH_RUNNER_BIN", current_exe);
    }
    builder
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .stdin(Stdio::null());

    let mut child = builder.spawn().map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("failed to spawn external server {:?}: {e}", cmd),
        )
    })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Fail::new(Code::RunFail, "external server has no stdout".to_string()))?;

    let port = tokio::task::spawn_blocking(move || -> Result<u16, String> {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.map_err(|e| format!("read stdout: {e}"))?;
            if let Some(rest) = line.strip_prefix("LISTENING port=") {
                return rest
                    .trim()
                    .parse::<u16>()
                    .map_err(|e| format!("invalid port in LISTENING line: {e}"));
            }
        }
        Err("external server exited without printing LISTENING port=<N>".to_string())
    })
    .await
    .map_err(|e| Fail::new(Code::RunFail, format!("port reader panicked: {e}")))?
    .map_err(|e| Fail::new(Code::RunFail, e))?;

    Ok((
        ServerHandle {
            port,
            stop: None,
            task: None,
            workers: Vec::new(),
            external_child: None,
            target_pid: None,
        },
        child,
    ))
}

fn resolve_cmd_token(token: &str) -> Result<String, Fail> {
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

fn resolve_arg_token(token: &str) -> Result<String, Fail> {
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
