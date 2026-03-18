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
pub async fn serve(cmd_json: &str) -> Result<(ServerHandle, Child), Fail> {
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

    let mut builder = Command::new(&cmd[0]);
    builder.args(&cmd[1..]);
    if let Some(dir) = &cwd {
        builder.current_dir(dir);
    }
    // Forward seed file path so external targets can read pre-generated data.
    // BENCH_SEED and BENCH_TRIAL are still passed for backwards compatibility.
    if let Ok(seed_file) = std::env::var("BENCH_SEED_FILE") {
        builder.env("BENCH_SEED_FILE", seed_file);
    }
    if let Ok(seed) = std::env::var("BENCH_SEED") {
        builder.env("BENCH_SEED", seed);
    }
    if let Ok(trial) = std::env::var("BENCH_TRIAL") {
        builder.env("BENCH_TRIAL", trial);
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
