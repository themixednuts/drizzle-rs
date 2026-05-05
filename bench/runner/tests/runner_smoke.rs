use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn checked_in_benchmark_specs_validate() {
    let root = workspace_root();
    validate_json_file(
        &root.join("bench/spec/workload.throughput.v1.json"),
        &root.join("docs/benchmark-spec/jsonschema/workload.v1.schema.json"),
    );
    validate_json_file(
        &root.join("bench/spec/workload.preview.v1.json"),
        &root.join("docs/benchmark-spec/jsonschema/workload.v1.schema.json"),
    );

    for path in [
        "bench/spec/targets.sqlite.v1.json",
        "bench/spec/targets.turso.v1.json",
        "bench/spec/targets.postgres.v1.json",
        "bench/spec/targets.postgres-rust-orms.v1.json",
        "bench/spec/targets.postgres-ts.v1.json",
        "bench/spec/targets.spacetimedb.v1.json",
    ] {
        validate_target_file(
            &root.join(path),
            &root.join("docs/benchmark-spec/jsonschema/target.v1.schema.json"),
        );
    }
}

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
            "--trials",
            "3",
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
    assert!(
        summary["spread"]["variance"]["rps"]["value"]
            .as_f64()
            .is_some()
    );

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest"),
    )
    .expect("manifest json");
    assert_eq!(manifest["runner"]["class"], "small");
    assert_eq!(manifest["name"], "Throughput HTTP (small)");
    assert!(
        manifest["cohort_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(manifest["runner"]["metrics"]["cpu_scope"], "host");
    assert_eq!(
        manifest["runner"]["metrics"]["memory_scope"],
        "target_process_tree"
    );
    assert_eq!(manifest["runner"]["metrics"]["network_scope"], "unmeasured");
    assert!(manifest["runner"]["headroom"].get("net_peak").is_none());
    assert!(
        manifest["target_meta"]
            .as_array()
            .is_some_and(|items| items.len() == 1)
    );
    assert!(
        manifest["queries"]
            .as_array()
            .is_some_and(|items| items.len() >= 10)
    );
    assert_eq!(manifest["trials"]["aggregate"], "median");
    assert!(manifest.get("compat").is_none());

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

#[test]
fn load_can_spawn_builtin_server_process() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    write_json(
        input.join("requests.json"),
        r#"[{"method":"GET","path":"/stats"}]"#,
    );

    let bin = assert_cmd::cargo::cargo_bin!("bench-runner");
    let server_cmd = serde_json::to_string(&vec![
        bin.to_string_lossy().to_string(),
        "serve".to_string(),
    ])
    .expect("server cmd json");

    let output = {
        let mut cmd = cargo_bin_cmd!("bench-runner");
        cmd.args([
            "load",
            "--target",
            "drizzle-rs-sqlite",
            "--trial",
            "1",
            "--seed",
            "42",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("workload.json").to_str().expect("workload path"),
            "--requests",
            input.join("requests.json").to_str().expect("requests path"),
            "--out",
            out.join("series.json").to_str().expect("series path"),
        ])
        .env("BENCH_SERVER_CMD", server_cmd);
        cmd.assert().success().get_output().clone()
    };
    assert_eq!(output.status.code(), Some(0));

    let series: Value =
        serde_json::from_str(&fs::read_to_string(out.join("series.json")).expect("series read"))
            .expect("series json");
    assert!(series.as_array().is_some_and(|points| !points.is_empty()));
}

fn run_cmd(args: &[&str], expect_success: bool) -> std::process::Output {
    let mut cmd = cargo_bin_cmd!("bench-runner");
    if matches!(args.first(), Some(&"run")) && !args.contains(&"--class") {
        cmd.args(args).args(["--class", "small"]);
    } else {
        cmd.args(args);
    }
    let assert = if expect_success {
        cmd.assert().success()
    } else {
        cmd.assert().failure()
    };
    assert.get_output().clone()
}

fn write_json(path: PathBuf, body: &str) {
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(path, body).expect("write file");
}

fn validate_json_file(path: &Path, schema_path: &Path) {
    let value: Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read spec")).expect("parse spec");
    let schema: Value =
        serde_json::from_str(&fs::read_to_string(schema_path).expect("read schema"))
            .expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("compile schema");
    let errors = validator
        .iter_errors(&value)
        .map(|err| err.to_string())
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "{}: {}",
        path.display(),
        errors.join("; ")
    );
}

fn validate_target_file(path: &Path, schema_path: &Path) {
    let value: Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read targets")).expect("parse");
    let schema: Value =
        serde_json::from_str(&fs::read_to_string(schema_path).expect("read schema"))
            .expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("compile target schema");
    let items = value.as_array().expect("targets must be an array");
    assert!(
        !items.is_empty(),
        "targets must not be empty: {}",
        path.display()
    );
    for (idx, item) in items.iter().enumerate() {
        let errors = validator
            .iter_errors(item)
            .map(|err| err.to_string())
            .collect::<Vec<_>>();
        assert!(
            errors.is_empty(),
            "{}[{idx}]: {}",
            path.display(),
            errors.join("; ")
        );
    }
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn workload_json(seed: u64) -> String {
    format!(
        r#"{{
  "version": "v1",
  "suite": "throughput-http",
  "load": {{
    "kind": "closed",
    "executor": "constant-vus",
    "unit": "1s",
    "concurrency": 1
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
      "vus": 1
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
    assert_eq!(summary["spread"]["variance"]["rps"]["samples"], 3);
    assert!(
        summary["spread"]["variance"]["p95"]["value"]
            .as_f64()
            .is_some()
    );

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
    assert!(
        result["cohort_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
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
    assert!(
        runs[0]["cohort_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(runs[0]["name"], "Throughput HTTP (small)");
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
    "display": {
      "name": "Drizzle RS SQLite",
      "description": "test target"
    },
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
    "parity": {
      "cmd": ["__BIN__", "parity"]
    },
    "load": {
      "cmd": ["__BIN__", "load"]
    }
  }
]"#
    .replace("__BIN__", &bin)
}
