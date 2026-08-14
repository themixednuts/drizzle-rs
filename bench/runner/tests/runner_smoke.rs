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
    validate_json_file(
        &root.join("bench/spec/workload.single-throughput.v1.json"),
        &root.join("docs/benchmark-spec/jsonschema/workload.v1.schema.json"),
    );
    validate_json_file(
        &root.join("bench/spec/workload.saturation.v1.json"),
        &root.join("docs/benchmark-spec/jsonschema/workload.v1.schema.json"),
    );
    validate_json_file(
        &root.join("bench/spec/workload.saturation-preview.v1.json"),
        &root.join("docs/benchmark-spec/jsonschema/workload.v1.schema.json"),
    );

    for path in [
        "bench/spec/targets.sqlite.v1.json",
        "bench/spec/targets.sqlite-ts.v1.json",
        "bench/spec/targets.libsql.v1.json",
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
    // This workload declares no capacity measurement, so it reports none.
    assert!(summary.get("saturation").is_none());
    // Nor a latency SLO, so no latency-under-SLO block either.
    assert!(summary.get("latency").is_none());
    // The n=5 bootstrap only ever measured its own resampling noise.
    assert!(
        summary["spread"].get("ci95").is_none(),
        "ci95 must no longer be emitted"
    );
    assert!(
        summary["spread"]["variance"]["rps"]["value"]
            .as_f64()
            .is_some()
    );
    // p90 is measured now, not interpolated from avg and p95.
    assert!(summary["primary"]["latency"]["p50"].as_f64().is_some());
    assert!(summary["primary"]["latency"]["p90"].as_f64().is_some());

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest"),
    )
    .expect("manifest json");
    assert_eq!(manifest["runner"]["class"], "small");
    assert_eq!(manifest["name"], "Throughput HTTP (small)");
    assert_eq!(manifest["load"]["pacing"], "none");
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
        manifest["runner"]["headroom"]["cpu_mean_peak"]
            .as_f64()
            .is_some()
    );
    // Colocation is disclosed rather than implied.
    assert_eq!(manifest["runner"]["topology"]["loadgen_colocated"], true);
    assert!(manifest["runner"]["topology"].get("db_colocated").is_some());
    assert!(manifest["runner"]["topology"].get("cpu_pinning").is_some());
    // Real host info, not the target triple's arch string.
    assert!(
        manifest["runner"]["cpu"]
            .as_str()
            .is_some_and(|cpu| !cpu.is_empty())
    );
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
            .join("0.series.json")
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
            "0",
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
    let points = series.as_array().expect("series array");
    assert!(!points.is_empty());
    // Every bucket carries its provenance so aggregation can restrict to steady
    // state and the dashboard can segment on trial boundaries.
    for point in points {
        // Echoes --trial so the aggregator can segment concatenated series.
        assert_eq!(point["trial"], 0);
        assert!(point["stage"].as_u64().is_some());
        assert!(
            ["warmup", "ramp", "hold"].contains(&point["phase"].as_str().expect("phase")),
            "unexpected phase {:?}",
            point["phase"]
        );
        assert!(point["requests"].as_u64().is_some());
        assert!(point["latency"]["p50"].as_f64().is_some());
        assert!(point["latency"]["p90"].as_f64().is_some());
    }
}

/// Every builtin embedded target must resolve, serve the whole route contract,
/// and pass the value-level parity checks.
#[test]
fn builtin_embedded_targets_serve_and_pass_parity() {
    for target in [
        "drizzle-rs-sqlite",
        "drizzle-rs-sqlite-query",
        "rusqlite-sqlite-prepared",
        "rusqlite-sqlite-unprepared",
        "drizzle-rs-turso",
    ] {
        assert_parity(target);
    }
}

/// The libsql family is feature-gated, so its ids only exist — and only need
/// proving — in a build that linked the driver.
#[cfg(feature = "libsql")]
#[test]
fn builtin_libsql_targets_serve_and_pass_parity() {
    for target in [
        "drizzle-rs-libsql",
        "libsql-sqlite-prepared",
        "libsql-sqlite-unprepared",
    ] {
        assert_parity(target);
    }
}

/// Without the feature the ids must still be recognised, and rejected with a
/// message that names the missing feature rather than "unsupported target".
#[cfg(not(feature = "libsql"))]
#[test]
fn libsql_targets_report_the_missing_feature() {
    for target in [
        "drizzle-rs-libsql",
        "libsql-sqlite-prepared",
        "libsql-sqlite-unprepared",
    ] {
        let mut cmd = cargo_bin_cmd!("bench-runner");
        cmd.args(["parity", "--target", target, "--seed", "42"]);
        let output = cmd.assert().failure().get_output().clone();
        let stderr = String::from_utf8(output.stderr).expect("utf8");
        assert!(stderr.contains("--features libsql"), "{target}: {stderr}");
    }
}

/// Start a builtin target, exercise all fourteen routes, and assert the
/// value-level parity checks.
///
/// `parity` is the cheapest end-to-end proof that a target id is wired up: it
/// asserts row counts, pagination and aggregate consistency against real
/// seeded data.
fn assert_parity(target: &str) {
    let mut cmd = cargo_bin_cmd!("bench-runner");
    cmd.args(["parity", "--target", target, "--seed", "42"]);
    let output = cmd.assert().get_output().clone();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{target}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unknown_target_ids_are_rejected() {
    let mut cmd = cargo_bin_cmd!("bench-runner");
    cmd.args(["parity", "--target", "not-a-target", "--seed", "42"]);
    let output = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(stderr.contains("unsupported target"), "{stderr}");
}

#[test]
fn arrival_rate_executors_are_rejected() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    // The schema still lists the name, but the runner has no open-loop
    // generator; accepting it would silently run a closed-loop VU test.
    write_json(
        input.join("workload.json"),
        &workload_json(17).replace("\"constant-vus\"", "\"constant-arrival-rate\""),
    );
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
        ],
        false,
    );

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(stderr.contains("not implemented"), "{stderr}");
}

#[test]
fn baseline_requires_matching_workload_and_class() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &workload_json(17));
    // Same suite, different workload content — and therefore a different hash.
    write_json(input.join("other.json"), &workload_json(99));
    write_json(input.join("targets.json"), &targets_json());
    write_json(input.join("requests.json"), r#"[]"#);

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

    // Comparing a different workload against it must refuse rather than call
    // the difference a regression.
    let mismatched = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("other.json").to_str().expect("workload path"),
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
        false,
    );
    assert_eq!(mismatched.status.code(), Some(10));
    let stderr = String::from_utf8(mismatched.stderr).expect("utf8");
    assert!(stderr.contains("workload mismatch"), "{stderr}");

    // `auto` must not silently pick the incompatible run either.
    let auto = run_cmd(
        &[
            "run",
            "--suite",
            "throughput-http",
            "--workload",
            input.join("other.json").to_str().expect("workload path"),
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
    let auto_id = extract_run_id(&auto);
    let report = fs::read_to_string(
        out.join("runs")
            .join(&auto_id)
            .join("reports")
            .join("compare.md"),
    )
    .expect("compare report read");
    assert!(
        !report.contains("Baseline:"),
        "auto baseline must skip incompatible runs: {report}"
    );
}

#[test]
fn run_publish_flag_updates_the_index() {
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
            "--publish",
        ],
        true,
    );
    let run_id = extract_run_id(&output);

    // --publish used to log "done (artifact stage only)" and write nothing.
    let index: Value =
        serde_json::from_str(&fs::read_to_string(out.join("index.json")).expect("index read"))
            .expect("index json");
    let runs = index["runs"].as_array().expect("runs array");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["run_id"], run_id);
}

/// `fair.family`, `target_meta[].fair.family` and `harness[].family` are one key
/// space. Consumers key harness lookup on it, so drift between the schemas would
/// silently render every affected row as "harness not declared" — a false
/// negative on exactly the disclosure the block exists to provide.
#[test]
fn the_family_vocabulary_is_one_key_space() {
    let root = workspace_root();
    let target_schema =
        read_json(&root.join("docs/benchmark-spec/jsonschema/target.v1.schema.json"));
    let manifest_schema =
        read_json(&root.join("docs/benchmark-spec/jsonschema/run-manifest.v1.schema.json"));

    let vocabulary = |schema: &Value| -> Vec<String> {
        schema["$defs"]["family"]["enum"]
            .as_array()
            .expect("family enum")
            .iter()
            .map(|value| value.as_str().expect("family id").to_string())
            .collect()
    };
    let declared = vocabulary(&target_schema);
    assert_eq!(
        declared,
        vocabulary(&manifest_schema),
        "target.v1 and run-manifest.v1 family enums have drifted"
    );

    // Both manifest uses must resolve to that one definition rather than a
    // parallel inline copy that could rot independently.
    for pointer in [
        "/properties/harness/items/properties/family",
        "/$defs/fair/properties/family",
    ] {
        assert_eq!(
            manifest_schema.pointer(pointer).expect(pointer)["$ref"],
            "#/$defs/family",
            "{pointer} must reference the shared family definition"
        );
    }

    // Ids only. A human-readable label frozen into a published artifact can
    // never be reworded, so naming stays with the presentation layer.
    for id in &declared {
        assert!(
            id.chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'),
            "family id {id} looks like a display label"
        );
    }

    // Every checked-in target declares a family from that vocabulary.
    for path in [
        "bench/spec/targets.sqlite.v1.json",
        "bench/spec/targets.sqlite-ts.v1.json",
        "bench/spec/targets.libsql.v1.json",
        "bench/spec/targets.turso.v1.json",
        "bench/spec/targets.postgres.v1.json",
        "bench/spec/targets.postgres-rust-orms.v1.json",
        "bench/spec/targets.postgres-ts.v1.json",
        "bench/spec/targets.spacetimedb.v1.json",
    ] {
        for target in read_json(&root.join(path)).as_array().expect("targets") {
            let family = target["fair"]["family"].as_str().expect("fair.family");
            assert!(
                declared.iter().any(|id| id == family),
                "{path}: {family} is not in the family vocabulary"
            );
        }
    }
}

/// A saturation workload must produce a schema-valid capacity artifact end to
/// end: a real curve tagged with concurrency, a named outcome, and a peak that
/// is one of the plotted steps.
#[test]
fn a_saturation_run_writes_a_capacity_artifact() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &saturation_workload_json());
    write_json(input.join("targets.json"), &targets_json());
    write_json(
        input.join("requests.json"),
        r#"[{"method":"GET","path":"/customer-by-id"}]"#,
    );

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
            "2",
            "--seed",
            "42",
        ],
        true,
    );
    let run_dir = out.join("runs").join(extract_run_id(&output));

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

    let saturation = &summary["saturation"];
    assert_eq!(saturation["slo"]["metric"], "p99");
    let outcome = saturation["outcome"].as_str().expect("outcome is required");
    assert!(
        ["saturated", "slo_never_met", "did_not_saturate"].contains(&outcome),
        "unexpected outcome {outcome}"
    );

    // The curve is the whole ramp: one entry per declared step, ascending, each
    // carrying the keys consumers rely on being present.
    let curve = saturation["curve"].as_array().expect("curve array");
    assert_eq!(
        curve
            .iter()
            .map(|step| step["concurrency"].as_u64().expect("concurrency"))
            .collect::<Vec<_>>(),
        vec![2, 4, 8]
    );
    for step in curve {
        assert!(step["slo_met"].is_boolean(), "slo_met must be present");
        // `null`, never omitted — consumers key off the field existing.
        assert!(
            step.get("disqualified").is_some(),
            "disqualified must be present on every step"
        );
        assert!(step["latency"]["p99"].as_f64().is_some());
        assert!(step["rps"].as_f64().is_some());
    }

    // Exactly one of peak / lower_bound_rps, decided by the outcome.
    match outcome {
        "saturated" => {
            let concurrency = saturation["peak"]["concurrency"]
                .as_u64()
                .expect("a saturated run carries a peak");
            assert!(saturation.get("lower_bound_rps").is_none());
            assert!(
                curve
                    .iter()
                    .any(|step| step["concurrency"].as_u64() == Some(concurrency)),
                "peak concurrency must be one of the plotted steps"
            );
        }
        "did_not_saturate" => {
            assert!(saturation.get("peak").is_none());
            assert!(saturation["lower_bound_rps"].as_f64().is_some());
        }
        _ => {
            assert!(saturation.get("peak").is_none());
            assert!(saturation.get("lower_bound_rps").is_none());
        }
    }

    // Per-step raw artifacts are kept: the curve has to be re-derivable.
    assert!(
        run_dir
            .join("targets")
            .join("drizzle-rs-sqlite")
            .join("raw")
            .join("trial")
            .join("0.steps.json")
            .exists()
    );

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest"),
    )
    .expect("manifest json");
    let harness = manifest["harness"].as_array().expect("harness array");
    assert_eq!(harness.len(), 1);
    assert_eq!(harness[0]["family"], "sqlite");
    assert_eq!(harness[0]["pool"], 1);
    assert_eq!(harness[0]["within_family_identical"], true);
    assert!(harness[0]["tuning"].as_str().is_some());
    // The warning list it replaced is gone, not emitted alongside it.
    assert!(manifest.get("fairness").is_none());

    // The emitted harness keys must be the same strings the targets declared:
    // consumers join the two, and a drifted key reads as "harness not declared"
    // on every affected row rather than failing loudly.
    let declared: Vec<&str> = manifest["target_meta"]
        .as_array()
        .expect("target_meta")
        .iter()
        .map(|meta| meta["fair"]["family"].as_str().expect("fair.family"))
        .collect();
    for block in harness {
        let family = block["family"].as_str().expect("harness family");
        assert!(
            declared.contains(&family),
            "harness family {family} matches no target's fair.family {declared:?}"
        );
        for id in block["targets"].as_array().expect("targets") {
            let id = id.as_str().expect("target id");
            assert!(
                manifest["targets"]
                    .as_array()
                    .expect("targets")
                    .iter()
                    .any(|t| t == id),
                "harness names target {id}, which did not run"
            );
        }
    }

    let validate = run_cmd(
        &["validate", "--run", run_dir.to_str().expect("run path")],
        true,
    );
    assert_eq!(validate.status.code(), Some(0));
}

/// A workload that declared no capacity measurement must not report one. The
/// removed heuristic emitted `saturation` unconditionally, which is how a
/// number bounded by the load generator's own sleep timer ended up published
/// under a capacity name.
#[test]
fn a_run_without_a_saturation_spec_emits_no_saturation_block() {
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
            "1",
            "--seed",
            "42",
        ],
        true,
    );

    let summary: Value = serde_json::from_str(
        &fs::read_to_string(
            out.join("runs")
                .join(extract_run_id(&output))
                .join("targets")
                .join("drizzle-rs-sqlite")
                .join("summary.json"),
        )
        .expect("summary read"),
    )
    .expect("summary json");
    assert!(
        summary.get("saturation").is_none(),
        "a workload that declared no capacity measurement must not report one"
    );
}

/// Within a family the harness must be identical, and the failure has to name
/// both targets and the field so the fix is obvious.
#[test]
fn within_family_harness_drift_fails_the_run_before_any_load() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    // Same family, different pool: exactly the comparison the check exists for.
    let drifted =
        targets_json().replace(r#""id": "drizzle-rs-sqlite","#, r#""id": "other-sqlite","#);
    let drifted = drifted.replace(r#""pool": 1,"#, r#""pool": 4,"#);
    let drifted = drifted.replace(r#""max": 1 }"#, r#""max": 4 }"#);
    let mut both: Vec<Value> = serde_json::from_str(&targets_json()).expect("targets");
    both.extend(serde_json::from_str::<Vec<Value>>(&drifted).expect("drifted"));
    write_json(
        input.join("targets.json"),
        &serde_json::to_string(&both).expect("targets json"),
    );
    write_json(input.join("workload.json"), &workload_json(17));
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
        ],
        false,
    );

    assert_eq!(output.status.code(), Some(3), "invalid_input");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(stderr.contains("unfair sqlite comparison"), "{stderr}");
    assert!(stderr.contains("other-sqlite"), "{stderr}");
    assert!(stderr.contains("drizzle-rs-sqlite"), "{stderr}");
    assert!(stderr.contains("fair.pool=4"), "{stderr}");
    assert!(stderr.contains("fair.pool=1"), "{stderr}");
    // It fails before spending an hour producing an unusable comparison.
    assert!(!out.join("runs").exists() || stderr.contains("fair.pool"));
}

/// A workload declaring the latency measurement must produce a schema-valid
/// sustained-latency artifact end to end: a real curve tagged with concurrency
/// and the criterion's own inputs, a named outcome, the fixed reference step,
/// and probe buckets disclosed in the timeseries.
#[test]
fn a_latency_run_writes_a_sustained_latency_artifact() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let input = root.join("input");
    let out = root.join("out");
    fs::create_dir_all(&input).expect("mkdir input");
    fs::create_dir_all(&out).expect("mkdir out");

    write_json(input.join("workload.json"), &latency_workload_json());
    write_json(input.join("targets.json"), &targets_json());
    write_json(
        input.join("requests.json"),
        r#"[{"method":"GET","path":"/customer-by-id"}]"#,
    );

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
            "2",
            "--seed",
            "42",
        ],
        true,
    );
    let run_dir = out.join("runs").join(extract_run_id(&output));

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

    // primary.latency is untouched: it stays the whole-ramp aggregate for
    // comparability, and the new block sits beside it.
    assert!(summary["primary"]["latency"]["p95"].as_f64().is_some());
    // A latency workload without a saturation block reports no capacity.
    assert!(summary.get("saturation").is_none());

    let latency = &summary["latency"];
    assert!(latency["tolerance"].as_f64().is_some_and(|t| t > 0.0));
    let outcome = latency["outcome"].as_str().expect("outcome is required");
    assert!(
        ["measured", "floor_above_knee", "floor_disqualified"].contains(&outcome),
        "unexpected outcome {outcome}"
    );

    let curve = latency["curve"].as_array().expect("curve array");
    assert_eq!(
        curve
            .iter()
            .map(|step| step["concurrency"].as_u64().expect("concurrency"))
            .collect::<Vec<_>>(),
        vec![2, 4, 8]
    );
    for step in curve {
        // The criterion's inputs travel with every step so the reading is
        // auditable from the artifact alone.
        assert!(step["offered_rps"].as_f64().is_some());
        assert!(step["retention"].as_f64().is_some());
        assert!(step["sustained"].is_boolean(), "sustained must be present");
        assert!(
            step.get("disqualified").is_some(),
            "disqualified must be present on every step"
        );
        assert!(step["latency"]["p95"].as_f64().is_some());
        assert!(step["rps"].as_f64().is_some());
    }
    // The floor is its own reference.
    assert_eq!(curve[0]["retention"].as_f64(), Some(1.0));

    // The reference exists unless the floor could not be corroborated, and it
    // is always the ladder's second rung — a fixed reading point, never a
    // knee-dependent one.
    if outcome.starts_with("floor_") {
        assert!(latency.get("reference").is_none());
    } else {
        assert_eq!(
            latency["reference"]["concurrency"].as_u64(),
            Some(4),
            "the reference is the ladder's second rung"
        );
    }

    // Probe buckets are disclosed in the timeseries and excluded nowhere else:
    // the floor stage's hold buckets carry the flag, counted stages never do.
    let series: Value = serde_json::from_str(
        &fs::read_to_string(
            run_dir
                .join("targets")
                .join("drizzle-rs-sqlite")
                .join("timeseries.json"),
        )
        .expect("timeseries read"),
    )
    .expect("timeseries json");
    let points = series["points"].as_array().expect("points");
    assert!(
        points
            .iter()
            .any(|point| point["probe"].as_bool() == Some(true)),
        "probe buckets must be disclosed"
    );
    for point in points {
        let vus = point["vus"].as_u64();
        match point["probe"].as_bool() {
            Some(true) => assert_eq!(vus, Some(2), "only the floor stage is a probe"),
            _ => assert_ne!(vus, Some(2), "the probe stage must carry its flag"),
        }
    }

    let validate = run_cmd(
        &["validate", "--run", run_dir.to_str().expect("run path")],
        true,
    );
    assert_eq!(validate.status.code(), Some(0));
}

/// Warmup at 2 VUs (stages 0-1), a probe floor at 2, then counted steps at 4
/// and 8. The outcome depends on this machine's speed; the test asserts
/// artifact shape and invariants, not a particular reading.
fn latency_workload_json() -> String {
    r#"{
  "version": "v1",
  "suite": "throughput-http",
  "name": "Latency",
  "load": { "kind": "closed", "executor": "ramping-vus", "unit": "1s", "concurrency": 8 },
  "data": { "name": "base", "seed": 42, "schema": "bench/schema.sql" },
  "shape": { "mode": "single", "endpoint": "/customer-by-id" },
  "stages": [
    { "sec": 1, "vus": 2, "probe": true },
    { "sec": 1, "vus": 2, "probe": true },
    { "sec": 2, "vus": 2, "probe": true },
    { "sec": 1, "vus": 4 },
    { "sec": 2, "vus": 4 },
    { "sec": 1, "vus": 8 },
    { "sec": 2, "vus": 8 }
  ],
  "warmup_s": 2,
  "requests": { "source": "generated", "file": "requests.json", "skip": [] },
  "pacing": { "mode": "none" },
  "sampling": { "cpu_ms": 100, "bucket_s": 1 },
  "limits": { "err": 0.01 },
  "latency": {}
}"#
    .to_string()
}

/// Warmup at 2 VUs (stages 0-1), then steps at 2, 4 and 8.
fn saturation_workload_json() -> String {
    r#"{
  "version": "v1",
  "suite": "throughput-http",
  "name": "Saturation",
  "load": { "kind": "closed", "executor": "ramping-vus", "unit": "1s", "concurrency": 8 },
  "data": { "name": "base", "seed": 42, "schema": "bench/schema.sql" },
  "shape": { "mode": "single", "endpoint": "/customer-by-id" },
  "stages": [
    { "sec": 1, "vus": 2 },
    { "sec": 1, "vus": 2 },
    { "sec": 2, "vus": 2 },
    { "sec": 1, "vus": 4 },
    { "sec": 2, "vus": 4 },
    { "sec": 1, "vus": 8 },
    { "sec": 2, "vus": 8 }
  ],
  "warmup_s": 2,
  "requests": { "source": "generated", "file": "requests.json", "skip": [] },
  "pacing": { "mode": "none" },
  "sampling": { "cpu_ms": 100, "bucket_s": 1 },
  "limits": { "err": 0.01 },
  "saturation": { "slo": { "metric": "p99", "ms": 250 } }
}"#
    .to_string()
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

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read json")).expect("parse json")
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
  "name": "Throughput HTTP",
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
  "warmup_s": 0,
  "requests": {{
    "source": "generated",
    "file": "requests.json",
    "skip": []
  }},
  "pacing": {{
    "mode": "none"
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
fn multi_trial_produces_spread_and_boxplot() {
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
    assert!(summary["spread"]["boxplot"]["rps"].is_object());
    assert!(summary["spread"].get("ci95").is_none());

    // Check raw trial files exist
    let trial_dir = run_dir
        .join("targets")
        .join("drizzle-rs-sqlite")
        .join("raw")
        .join("trial");
    // Trial indices are zero-based everywhere: BENCH_TRIAL, Point.trial, and
    // the raw artifact filenames all use the same number.
    assert!(trial_dir.join("0.series.json").exists());
    assert!(trial_dir.join("1.series.json").exists());
    assert!(trial_dir.join("2.series.json").exists());
    assert!(!trial_dir.join("3.series.json").exists());
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
      "family": "sqlite",
      "workers": 1,
      "pool": 1,
      "db": "sqlite",
      "schema": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
      "contract": "v1",
      "tuning": "WAL journal, temp_store=MEMORY"
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
