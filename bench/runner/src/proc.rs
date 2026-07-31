//! Process-group and CPU-affinity helpers.
//!
//! Benchmark children (target servers, per-target `load`/`parity` commands)
//! frequently spawn grandchildren — `bun run`, `cargo run`, shell wrappers.
//! Killing only the direct child leaves the real server holding its listening
//! socket, which poisons every following trial. Everything the runner spawns
//! therefore goes into its own process group so it can be torn down as a unit.

use std::process::{Child, Command};

/// Put `cmd` into a fresh process group so the whole subtree can be signalled
/// together. No-op on platforms without process groups.
pub fn spawn_in_own_group(cmd: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: `setsid` is async-signal-safe and touches no allocator state.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    // Already a group leader is fine; anything else is fatal for
                    // the group-kill guarantee, so surface it.
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() != Some(libc::EPERM) {
                        return Err(err);
                    }
                }
                Ok(())
            });
        }
    }
    #[cfg(not(unix))]
    {
        let _ = cmd;
    }
}

/// Additionally pin the child to `BENCH_CPUSET_SERVER` when set (Linux only).
pub fn spawn_server_child(cmd: &mut Command) {
    spawn_in_own_group(cmd);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        let Some(cpus) = server_cpuset() else {
            return;
        };
        // SAFETY: `sched_setaffinity` is async-signal-safe.
        unsafe {
            cmd.pre_exec(move || {
                apply_affinity(&cpus);
                Ok(())
            });
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = cmd;
    }
}

/// Kill a child and everything it spawned.
pub fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as libc::pid_t;
        // SAFETY: plain libc call; an invalid pid just returns ESRCH.
        unsafe {
            libc::killpg(pid, libc::SIGTERM);
        }
        // Give the group a moment to unwind, then make sure it is gone.
        for _ in 0..50 {
            match child.try_wait() {
                Ok(Some(_)) => break,
                _ => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
        // SAFETY: same as above.
        unsafe {
            libc::killpg(pid, libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        // `taskkill /T` walks the child tree; fall back to a direct kill if the
        // tool is unavailable (e.g. stripped container images).
        let killed = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !killed {
            let _ = child.kill();
        }
    }
    let _ = child.wait();
}

/// Pin the current process to `BENCH_CPUSET_LOAD` (Linux only, no-op elsewhere).
/// Returns the applied spec so it can be recorded in the manifest topology.
pub fn pin_self_to_load_cpuset() -> Option<String> {
    let spec = std::env::var("BENCH_CPUSET_LOAD").ok()?;
    let cpus = parse_cpuset(&spec)?;
    #[cfg(target_os = "linux")]
    {
        apply_affinity(&cpus);
        return Some(spec);
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = cpus;
        None
    }
}

/// Human-readable description of the configured pinning, for the manifest.
pub fn cpu_pinning_description() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let load = std::env::var("BENCH_CPUSET_LOAD")
        .ok()
        .filter(|spec| parse_cpuset(spec).is_some());
    let server = std::env::var("BENCH_CPUSET_SERVER")
        .ok()
        .filter(|spec| parse_cpuset(spec).is_some());
    match (load, server) {
        (None, None) => None,
        (load, server) => {
            let mut parts = Vec::new();
            if let Some(load) = load {
                parts.push(format!("load={load}"));
            }
            if let Some(server) = server {
                parts.push(format!("server={server}"));
            }
            Some(parts.join(" "))
        }
    }
}

#[cfg(target_os = "linux")]
fn server_cpuset() -> Option<Vec<usize>> {
    parse_cpuset(&std::env::var("BENCH_CPUSET_SERVER").ok()?)
}

#[cfg(target_os = "linux")]
fn apply_affinity(cpus: &[usize]) {
    // SAFETY: `set` is zeroed before use and every index is bounds-checked
    // against CPU_SETSIZE by `CPU_SET`'s contract (we mask it ourselves).
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        for &cpu in cpus {
            if cpu < libc::CPU_SETSIZE as usize {
                libc::CPU_SET(cpu, &mut set);
            }
        }
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
    }
}

/// Parse a cpuset spec such as `0-1`, `2`, or `0-1,4`. Returns `None` when the
/// spec is malformed or empty so callers can treat it as "not configured".
pub fn parse_cpuset(spec: &str) -> Option<Vec<usize>> {
    let mut cpus = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('-') {
            Some((lo, hi)) => {
                let lo: usize = lo.trim().parse().ok()?;
                let hi: usize = hi.trim().parse().ok()?;
                if hi < lo {
                    return None;
                }
                cpus.extend(lo..=hi);
            }
            None => cpus.push(part.parse().ok()?),
        }
    }
    if cpus.is_empty() { None } else { Some(cpus) }
}

#[cfg(test)]
mod tests {
    use super::parse_cpuset;

    #[test]
    fn cpuset_parses_ranges_and_lists() {
        assert_eq!(parse_cpuset("0-1"), Some(vec![0, 1]));
        assert_eq!(parse_cpuset("3"), Some(vec![3]));
        assert_eq!(parse_cpuset("0-1,4"), Some(vec![0, 1, 4]));
        assert_eq!(parse_cpuset(" 2 - 3 "), Some(vec![2, 3]));
    }

    #[test]
    fn cpuset_rejects_garbage() {
        assert_eq!(parse_cpuset(""), None);
        assert_eq!(parse_cpuset("a-b"), None);
        assert_eq!(parse_cpuset("5-1"), None);
    }
}
