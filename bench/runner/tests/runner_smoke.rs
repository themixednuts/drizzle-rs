use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn run_writes_contract_artifacts() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

    let output = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--seed",
            "42",
        ],
        true,
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let run_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("run_id="))
        .expect("run_id line")
        .to_string();

    let run_dir = out.join("runs").join(run_id);
    assert!(run_dir.join("manifest.json").exists());
    assert!(run_dir.join("result.json").exists());
    assert!(run_dir.join("requests.generated.json").exists());
    assert!(
        run_dir
            .join("targets")
            .join("drizzle-rs-sqlite")
            .join("summary.json")
            .exists()
    );

    let summary: Value = serde_json::from_str(
        &fs::read_to_string(
            run_dir
                .join("targets")
                .join("drizzle-rs-sqlite")
                .join("summary.json"),
        )
        .expect("summary read"),
    )
    .expect("summary json");
    assert!(summary.get("spread").is_some());
    assert!(summary.get("saturation").is_some());
    assert!(summary["spread"].get("ci95").is_some());

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest"),
    )
    .expect("manifest json");
    assert_eq!(manifest["runner"]["class"], "publish");
    assert_eq!(manifest["trials"]["aggregate"], "median");
    assert_eq!(manifest["compat"]["class"], manifest["runner"]["class"]);
    assert_eq!(manifest["compat"]["workload"], manifest["workload"]);

    let parquet = fs::read(
        run_dir
            .join("targets")
            .join("drizzle-rs-sqlite")
            .join("raw")
            .join("k6.parquet"),
    )
    .expect("parquet read");
    assert!(parquet.len() > 8);
    assert_eq!(&parquet[0..4], b"PAR1");
    assert!(
        run_dir
            .join("targets")
            .join("drizzle-rs-sqlite")
            .join("raw")
            .join("trial")
            .join("1.series.json")
            .exists()
    );

    let validate = run_cmd(
        &[
            "validate",
            "--run",
            run_dir.to_str().expect("run path"),
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
        ],
        true,
    );
    assert_eq!(validate.status.code(), Some(0));
}

#[test]
fn missing_baseline_exits_no_baseline_code() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

    let output = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--baseline",
            "missing_run",
        ],
        false,
    );

    assert_eq!(output.status.code(), Some(10));
}

#[test]
fn publish_uses_workload_seed_not_cli_seed() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

    let one = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--class",
            "publish",
            "--seed",
            "42",
        ],
        true,
    );
    let two = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--class",
            "publish",
            "--seed",
            "99",
        ],
        true,
    );

    let left = generated_requests(&out, &one);
    let right = generated_requests(&out, &two);
    assert_eq!(left, right);
}

#[test]
fn capture_writes_telemetry() {
    let tmp = TempDir::new().expect("tmp");
    let out = tmp.path().join("telemetry");
    fs::create_dir_all(&out).expect("mkdir out");

    let output = run_cmd(
        &[
            "capture",
            "--out",
            out.to_str().expect("out path"),
            "--ms",
            "100",
            "--",
            "rustc",
            "--version",
        ],
        true,
    );
    assert_eq!(output.status.code(), Some(0));

    assert!(out.join("host.json").exists());
    assert!(out.join("summary.json").exists());

    let samples = fs::read_to_string(out.join("samples.jsonl")).expect("samples read");
    assert!(!samples.trim().is_empty());
}

fn run_cmd(args: &[&str], expect_success: bool) -> std::process::Output {
    let mut cmd = cargo_bin_cmd!("bench-runner");
    cmd.args(args);
    let assert = if expect_success {
        cmd.assert().success()
    } else {
        cmd.assert().failure()
    };
    assert.get_output().clone()
}

fn generated_requests(out: &Path, output: &std::process::Output) -> Value {
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8");
    let run_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("run_id="))
        .expect("run_id line")
        .to_string();
    let body = fs::read_to_string(
        out.join("runs")
            .join(run_id)
            .join("requests.generated.json"),
    )
    .expect("read requests");
    serde_json::from_str(&body).expect("requests json")
}

fn write_json(path: PathBuf, body: &str) {
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(path, body).expect("write file");
}

fn workload_json(seed: u64) -> String {
    format!(
        r#"{{
  "version": "v1",
  "suite": "throughput-http",
  "load": {{
    "kind": "open",
    "executor": "constant-arrival-rate",
    "unit": "1s"
  }},
  "data": {{
    "name": "base",
    "seed": {seed},
    "schema": "bench/schema.sql"
  }},
  "shape": {{
    "mode": "mixed",
    "endpoint": null
  }},
  "stages": [
    {{
      "sec": 1,
      "rps": 100.0
    }}
  ],
  "requests": {{
    "source": "generated",
    "file": "requests.json",
    "skip": []
  }},
  "sampling": {{
    "cpu_ms": 100,
    "bucket_s": 1
  }},
  "limits": {{
    "err": 0.01
  }}
}}"#
    )
}

#[test]
fn baseline_comparison_includes_deltas() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

    // First run — becomes the baseline
    let first = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--seed",
            "42",
        ],
        true,
    );
    let first_id = extract_run_id(&first);

    // Second run — using first as baseline
    let second = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--seed",
            "42",
            "--baseline",
            &first_id,
        ],
        true,
    );
    let second_id = extract_run_id(&second);

    let report = fs::read_to_string(
        out.join("runs")
            .join(&second_id)
            .join("reports")
            .join("compare.md"),
    )
    .expect("compare report read");
    assert!(report.contains("drizzle-rs-sqlite"));
    // Delta columns should have numeric values, not dashes (baseline was found)
    let data_line = report
        .lines()
        .find(|l| l.contains("drizzle-rs-sqlite"))
        .expect("data line");
    assert!(!data_line.ends_with("- | - | - | - |"));
}

#[test]
fn auto_baseline_discovers_latest_run() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

    // First run
    let first = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--seed",
            "42",
        ],
        true,
    );
    assert!(first.status.success());

    // Second run with --baseline auto
    let second = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--seed",
            "42",
            "--baseline",
            "auto",
        ],
        true,
    );
    let second_id = extract_run_id(&second);

    // Check that compare report has actual baseline data (not dashes)
    let report = fs::read_to_string(
        out.join("runs")
            .join(&second_id)
            .join("reports")
            .join("compare.md"),
    )
    .expect("compare report read");
    assert!(report.contains("Baseline:"));
}

#[test]
fn multi_trial_produces_spread_and_ci() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

    let output = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--trials",
            "3",
            "--seed",
            "42",
        ],
        true,
    );
    let run_id = extract_run_id(&output);
    let run_dir = out.join("runs").join(&run_id);

    let summary: Value = serde_json::from_str(
        &fs::read_to_string(
            run_dir
                .join("targets")
                .join("drizzle-rs-sqlite")
                .join("summary.json"),
        )
        .expect("summary read"),
    )
    .expect("summary json");

    assert_eq!(summary["spread"]["trials"], 3);
    assert_eq!(summary["spread"]["aggregate"], "median");
    assert!(summary["spread"]["rps"]["min"].as_f64().is_some());
    assert!(summary["spread"]["rps"]["max"].as_f64().is_some());
    assert!(summary["spread"]["p95"]["min"].as_f64().is_some());

    // With 3 trials, bootstrap CI should be present
    assert!(summary["spread"]["ci95"].is_object());

    // Check raw trial files exist
    let trial_dir = run_dir
        .join("targets")
        .join("drizzle-rs-sqlite")
        .join("raw")
        .join("trial");
    assert!(trial_dir.join("1.series.json").exists());
    assert!(trial_dir.join("2.series.json").exists());
    assert!(trial_dir.join("3.series.json").exists());
}

#[test]
fn result_includes_limits_gate() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

    let output = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--seed",
            "42",
        ],
        true,
    );
    let run_id = extract_run_id(&output);

    let result: Value = serde_json::from_str(
        &fs::read_to_string(out.join("runs").join(&run_id).join("result.json"))
            .expect("read result"),
    )
    .expect("result json");
    assert!(result["gates"]["limits"].is_string());
    assert_eq!(result["gates"]["limits"], "pass");
}

#[test]
fn publish_updates_index() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

    // Run a benchmark
    let output = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--seed",
            "42",
        ],
        true,
    );
    let run_id = extract_run_id(&output);
    let run_dir = out.join("runs").join(&run_id);

    // Publish — creates index from scratch
    let index_path = root.join("index.json");
    let publish = run_cmd(
        &[
            "publish",
            "--run",
            run_dir.to_str().expect("run dir"),
            "--index",
            index_path.to_str().expect("index path"),
        ],
        true,
    );
    assert_eq!(publish.status.code(), Some(0));

    let index: Value = serde_json::from_str(&fs::read_to_string(&index_path).expect("index read"))
        .expect("index json");
    assert_eq!(index["version"], "v1");
    let runs = index["runs"].as_array().expect("runs array");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["run_id"], run_id);
    assert_eq!(runs[0]["suite"], "throughput-http");
    assert!(!runs[0]["targets"].as_array().expect("targets").is_empty());

    // Run another benchmark and publish again — index grows
    let output2 = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--targets",
            input.join("targets.json").to_str().expect("targets path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.to_str().expect("out path"),
            "--seed",
            "42",
        ],
        true,
    );
    let run_id2 = extract_run_id(&output2);
    let run_dir2 = out.join("runs").join(&run_id2);

    let publish2 = run_cmd(
        &[
            "publish",
            "--run",
            run_dir2.to_str().expect("run dir"),
            "--index",
            index_path.to_str().expect("index path"),
        ],
        true,
    );
    assert_eq!(publish2.status.code(), Some(0));

    let index2: Value = serde_json::from_str(&fs::read_to_string(&index_path).expect("index read"))
        .expect("index json");
    let runs2 = index2["runs"].as_array().expect("runs array");
    assert_eq!(runs2.len(), 2);

    // Republish same run — should deduplicate, not grow
    let publish3 = run_cmd(
        &[
            "publish",
            "--run",
            run_dir2.to_str().expect("run dir"),
            "--index",
            index_path.to_str().expect("index path"),
        ],
        true,
    );
    assert_eq!(publish3.status.code(), Some(0));

    let index3: Value = serde_json::from_str(&fs::read_to_string(&index_path).expect("index read"))
        .expect("index json");
    let runs3 = index3["runs"].as_array().expect("runs array");
    assert_eq!(runs3.len(), 2);
}

fn extract_run_id(output: &std::process::Output) -> String {
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8");
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("run_id="))
        .expect("run_id line")
        .to_string()
}

fn targets_json() -> String {
    let bin = assert_cmd::cargo::cargo_bin!("bench-runner");
    let bin = bin.to_string_lossy().replace('\\', "\\\\");
    r#"[
  {
    "version": "v1",
    "id": "drizzle-rs-sqlite",
    "lang": "rust",
    "runtime": { "name": "rustc", "ver": "1.91" },
    "orm": { "name": "drizzle-rs-sqlite", "ver": "0.1.5" },
    "driver": { "name": "rusqlite", "ver": "0.37.0" },
    "proc": { "mode": "single", "workers": 1 },
    "pool": { "max": 1 },
    "db": { "profile": "sqlite", "hash": "sha256:1111111111111111111111111111111111111111111111111111111111111111" },
    "wire": { "format": "json" },
    "fair": {
      "workers": 1,
      "pool": 1,
      "db": "sqlite",
      "schema": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
      "contract": "v1"
    },
    "contract": { "ver": "v1" },
    "load": {
      "cmd": ["__BIN__", "load"]
    }
  }
]"#
    .replace("__BIN__", &bin)
}
