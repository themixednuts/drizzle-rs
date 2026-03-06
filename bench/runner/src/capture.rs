use crate::cli::Capture;
use crate::code::{Code, Fail};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Networks, System};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Serialize)]
struct Host {
    version: &'static str,
    time: String,
    label: String,
    cmd: Vec<String>,
    os: String,
    kernel: String,
    host: String,
    cpus: usize,
    mem_kb: u64,
}

#[derive(Debug, Serialize)]
struct Sample {
    time: String,
    cpu: f64,
    mhz: f64,
    mem_used_kb: u64,
    mem_total_kb: u64,
    swap_used_kb: u64,
    net_rx_bytes: u64,
    net_tx_bytes: u64,
    net_rx_rate: f64,
    net_tx_rate: f64,
}

#[derive(Debug, Serialize)]
struct Summary {
    version: &'static str,
    start: String,
    end: String,
    secs: f64,
    code: i32,
    ok: bool,
    samples: u64,
}

pub fn run(args: Capture) -> Result<Code, Fail> {
    if args.ms == 0 {
        return Err(Fail::new(Code::InvalidCli, "--ms must be greater than 0"));
    }
    if args.cmd.is_empty() {
        return Err(Fail::new(Code::InvalidCli, "missing command"));
    }

    fs::create_dir_all(&args.out).map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("failed to create output dir {}: {e}", args.out.display()),
        )
    })?;

    let start = now_tag();
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut nets = Networks::new_with_refreshed_list();
    let mut prev = net_totals(&nets);
    let mut prev_t = Instant::now();

    let host = Host {
        version: "v1",
        time: start.clone(),
        label: args.label.clone().unwrap_or_else(|| "capture".to_string()),
        cmd: args.cmd.clone(),
        os: os_string(&sys),
        kernel: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
        host: System::host_name().unwrap_or_else(|| "unknown".to_string()),
        cpus: sys.cpus().len(),
        mem_kb: to_kb(sys.total_memory()),
    };
    write_json(args.out.join("host.json"), &host)?;

    let mut child = child_cmd(&args.cmd).spawn().map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("failed to spawn command {}: {e}", args.cmd.join(" ")),
        )
    })?;

    let samples_path = args.out.join("samples.jsonl");
    let samples_file = File::create(&samples_path).map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("failed to create {}: {e}", samples_path.display()),
        )
    })?;
    let mut out = BufWriter::new(samples_file);

    let tick = Duration::from_millis(args.ms);
    let wall = Instant::now();
    let mut count = 0_u64;
    loop {
        if let Some(status) = child.try_wait().map_err(|e| {
            Fail::new(
                Code::RunFail,
                format!("failed waiting for child process: {e}"),
            )
        })? {
            write_one(&mut out, &mut sys, &mut nets, &mut prev, &mut prev_t)?;
            count += 1;
            out.flush().map_err(|e| {
                Fail::new(
                    Code::RunFail,
                    format!("failed to flush {}: {e}", samples_path.display()),
                )
            })?;

            let end = now_tag();
            let code = status.code().unwrap_or(1);
            let summary = Summary {
                version: "v1",
                start,
                end,
                secs: wall.elapsed().as_secs_f64(),
                code,
                ok: status.success(),
                samples: count,
            };
            write_json(args.out.join("summary.json"), &summary)?;
            return if status.success() {
                Ok(Code::Success)
            } else {
                Err(Fail::new(
                    Code::RunFail,
                    format!("child failed with code {code}"),
                ))
            };
        }

        write_one(&mut out, &mut sys, &mut nets, &mut prev, &mut prev_t)?;
        count += 1;
        thread::sleep(tick);
    }
}

fn child_cmd(cmd: &[String]) -> Command {
    let mut out = Command::new(&cmd[0]);
    out.args(&cmd[1..]);
    out.stdin(Stdio::inherit());
    out.stdout(Stdio::inherit());
    out.stderr(Stdio::inherit());
    out
}

fn write_one(
    out: &mut BufWriter<File>,
    sys: &mut System,
    nets: &mut Networks,
    prev: &mut (u64, u64),
    prev_t: &mut Instant,
) -> Result<(), Fail> {
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    nets.refresh(true);

    let cpus = sys.cpus();
    let cpu = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|v| f64::from(v.cpu_usage())).sum::<f64>() / cpus.len() as f64
    };
    let mhz = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|v| v.frequency() as f64).sum::<f64>() / cpus.len() as f64
    };

    let now = Instant::now();
    let dt = now.duration_since(*prev_t).as_secs_f64().max(0.001);
    let totals = net_totals(nets);
    let rx_delta = totals.0.saturating_sub(prev.0);
    let tx_delta = totals.1.saturating_sub(prev.1);
    *prev = totals;
    *prev_t = now;

    let row = Sample {
        time: now_tag(),
        cpu,
        mhz,
        mem_used_kb: to_kb(sys.used_memory()),
        mem_total_kb: to_kb(sys.total_memory()),
        swap_used_kb: to_kb(sys.used_swap()),
        net_rx_bytes: totals.0,
        net_tx_bytes: totals.1,
        net_rx_rate: rx_delta as f64 / dt,
        net_tx_rate: tx_delta as f64 / dt,
    };
    let line = serde_json::to_string(&row)
        .map_err(|e| Fail::new(Code::RunFail, format!("sample encode failed: {e}")))?;
    out.write_all(line.as_bytes())
        .and_then(|_| out.write_all(b"\n"))
        .map_err(|e| Fail::new(Code::RunFail, format!("sample write failed: {e}")))
}

fn to_kb(bytes: u64) -> u64 {
    bytes / 1024
}

fn net_totals(nets: &Networks) -> (u64, u64) {
    let mut rx = 0_u64;
    let mut tx = 0_u64;
    for (_, data) in nets {
        rx = rx.saturating_add(data.total_received());
        tx = tx.saturating_add(data.total_transmitted());
    }
    (rx, tx)
}

fn os_string(_sys: &System) -> String {
    let name = System::name().unwrap_or_else(|| "unknown".to_string());
    let ver = System::os_version().unwrap_or_else(|| "unknown".to_string());
    let long = System::long_os_version().unwrap_or_else(|| "unknown".to_string());
    let distro = System::distribution_id().to_string();
    format!("{name} {ver} ({long}; {distro})")
}

fn now_tag() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> Result<(), Fail> {
    let path = path.as_ref();
    let body = serde_json::to_string_pretty(value).map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("serialize {} failed: {e}", path.display()),
        )
    })?;
    fs::write(path, body).map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("write {} failed: {e}", path.display()),
        )
    })
}
