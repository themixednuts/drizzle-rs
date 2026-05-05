use crate::cli::{Class, Cli, Cmd, Run, Validate};
use crate::code::{Code, Fail};
use crate::model::{
    Artifacts, AvgPeakDoc, BoxMetricDoc, BoxPlotDoc, CiDoc, DatasetSummary, Event, Exec, Gate,
    Gates, Headroom, LatencyDoc, Limits, LoadSummary, ManifestDoc, Point, PrimaryDoc, QueryDoc,
    QueryShapeDoc, RangeDoc, RequestDoc, ResultDoc, Runner, RunnerMetrics, SaturationDoc,
    SpreadDoc, Status, SummaryDoc, Target, TargetMetaDoc, TimeseriesDoc, TrialMeta, VarianceDoc,
    VarianceMetricDoc, Workload,
};
use crate::workload_terms::{CUSTOMER_SEARCH_TERMS, PRODUCT_SEARCH_TERMS};
use parquet::data_type::{ByteArray, ByteArrayType, DoubleType};
use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::schema::parser::parse_message_type;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, format_description};

pub async fn exec(cli: Cli) -> Result<Code, Fail> {
    match cli.cmd {
        Cmd::Run(args) => run(args),
        Cmd::Capture(args) => crate::capture::run(args),
        Cmd::Serve(args) => crate::load::serve(args).await,
        Cmd::Load(args) => crate::load::run(args).await,
        Cmd::Parity(args) => crate::parity::run(args).await,
        Cmd::SeedPostgres(args) => crate::load::seed_postgres(args).await,
        Cmd::Validate(args) => validate(args),
        Cmd::Publish(args) => crate::publish::run(args),
    }
}

fn run(args: Run) -> Result<Code, Fail> {
    validate_cli(&args)?;

    let input = load_input(&args)?;
    let baseline = resolve_baseline(args.baseline.as_deref(), &args.out, input.suite)?;
    let run_id = make_run_id(input.suite, &input.git);
    let cohort_id = resolve_cohort_id(args.cohort_id.as_deref(), &run_id)?;
    let run_dir = args.out.join("runs").join(&run_id);
    let start = now_tag();

    init_layout(&run_dir, &input.targets)?;
    let events_path = run_dir.join("events.jsonl");
    let mut events = new_events_writer(&events_path)?;

    emit(&mut events, args.json, "info", "validate", "start")?;
    emit(&mut events, args.json, "info", "validate", "ok")?;

    let seed = resolve_seed(args.class, args.seed, input.seed);
    if matches!(args.class, Class::Full | Class::Publish) && args.seed.is_some() {
        emit(
            &mut events,
            args.json,
            "info",
            "validate",
            "ignored --seed for full/publish class; using workload seed",
        )?;
    }

    let requests = materialize_requests(
        &run_dir,
        seed,
        input.requests.clone(),
        &input.request_skip,
        input.request_count_hint,
    )?;
    write_env(
        &run_dir,
        &args,
        input.suite,
        input.trials,
        requests.len(),
        &run_id,
        &cohort_id,
    )?;

    // Write seed metadata file (seed value + table counts) for external targets
    let seed_meta = serde_json::json!({
        "seed": seed,
        "customers": crate::load::SEED_CUSTOMERS,
        "employees": crate::load::SEED_EMPLOYEES,
        "orders": crate::load::SEED_ORDERS,
        "suppliers": crate::load::SEED_SUPPLIERS,
        "products": crate::load::SEED_PRODUCTS,
    });
    let seed_file = run_dir.join("seed.v1.json");
    write_json(seed_file.clone(), &seed_meta, Code::RunFail)?;

    run_parity(&input.targets, seed, &seed_file, &mut events, args.json)?;
    run_warmup(&input.targets, &mut events, args.json)?;
    let points = run_trials(
        &run_dir,
        input.suite,
        &input.targets,
        &mut events,
        args.json,
        input.trials,
        seed,
        &seed_file,
        &args.workload,
        &run_dir.join("requests.generated.json"),
    )?;
    let aggregate = run_aggregate(&run_dir, &run_id, &input, &points, &mut events, args.json)?;
    let gates = run_gates(
        args.class,
        &aggregate.headroom,
        &aggregate.summaries,
        baseline.as_ref(),
        &input.limits,
        &mut events,
        args.json,
    )?;
    write_compare_report(&run_dir, &aggregate.summaries, baseline.as_ref())?;
    let end = now_tag();
    let workload_for_manifest = load_json::<Workload>(&args.workload)?;
    write_manifest(
        &run_dir,
        &run_id,
        &input,
        ManifestContext {
            class: args.class,
            cohort_id: &cohort_id,
            headroom: &aggregate.headroom,
            start: &start,
            end: &end,
            seed,
            requests_count: requests.len(),
            requests: &requests,
            workload: &workload_for_manifest,
        },
    )?;
    crate::schema::validate_run(&run_dir)?;

    if args.publish {
        run_publish(&mut events, args.json)?;
    }

    let result = ResultDoc {
        version: "v1",
        run_id: run_id.clone(),
        cohort_id: cohort_id.clone(),
        status: Status::Success,
        suite: input.suite.to_string(),
        class: class_name(args.class).to_string(),
        trials: input.trials,
        targets: input.targets.len(),
        requests: requests.len(),
        gates,
    };
    write_json(run_dir.join("result.json"), &result, Code::AggregateFail)?;

    emit(
        &mut events,
        args.json,
        "info",
        "write",
        "wrote manifest, target artifacts, and result",
    )?;
    events
        .flush()
        .map_err(|e| Fail::new(Code::RunFail, format!("event flush failed: {e}")))?;

    println!("run_id={run_id}");
    println!("cohort_id={cohort_id}");
    Ok(Code::Success)
}

fn validate(args: Validate) -> Result<Code, Fail> {
    if let Some(path) = &args.workload {
        crate::schema::validate_workload(path)?;
    }
    if let Some(path) = &args.targets {
        crate::schema::validate_targets(path)?;
    }
    crate::schema::validate_run(&args.run)?;
    Ok(Code::Success)
}

struct RunInput {
    suite: &'static str,
    seed: u64,
    trials: u32,
    git: String,
    workload_hash: String,
    targets: Vec<Target>,
    requests: Vec<RequestDoc>,
    request_skip: Vec<String>,
    request_count_hint: usize,
    limits: Limits,
}

struct Aggregate {
    headroom: Headroom,
    summaries: BTreeMap<String, PrimaryDoc>,
}

struct ManifestContext<'a> {
    class: Class,
    cohort_id: &'a str,
    headroom: &'a Headroom,
    start: &'a str,
    end: &'a str,
    seed: u64,
    requests_count: usize,
    requests: &'a [RequestDoc],
    workload: &'a Workload,
}

struct TargetArtifacts<'a> {
    run_id: &'a str,
    suite: &'a str,
    target_id: &'a str,
    group: Option<&'a str>,
    measurements: &'a [TrialMeasurement],
    summary: &'a PrimaryDoc,
    spread: &'a SpreadDoc,
    saturation: &'a SaturationDoc,
}

#[derive(Debug, Clone)]
struct TrialMeasurement {
    aggregate: Point,
    series: Vec<Point>,
}

struct Baseline {
    run_id: String,
    summaries: BTreeMap<String, PrimaryDoc>,
}

const MIX_CUSTOMER_BY_ID: usize = 19_999;
const MIX_EMPLOYEE_WITH_RECIPIENT: usize = 5_000;
const MIX_SUPPLIER_BY_ID: usize = 30_000;
const MIX_PRODUCT_WITH_SUPPLIER: usize = 100_000;
const MIX_ORDER_WITH_DETAILS: usize = 100_000;
const MIX_ORDER_WITH_DETAILS_AND_PRODUCTS: usize = 100_000;
const MIX_CUSTOMERS: usize = 2_000;
const MIX_EMPLOYEES: usize = 1_000;
const MIX_SUPPLIERS: usize = 1_000;
const MIX_PRODUCTS: usize = 3_000;
const MIX_ORDERS_WITH_DETAILS: usize = 10_000;
const MIX_SEARCH_CUSTOMER: usize = 5_000;
const MIX_SEARCH_PRODUCT: usize = 50_000;

fn validate_cli(args: &Run) -> Result<(), Fail> {
    if let Some(trials) = args.trials
        && trials == 0
    {
        return Err(Fail::new(
            Code::InvalidCli,
            "--trials must be greater than 0",
        ));
    }
    Ok(())
}

fn load_input(args: &Run) -> Result<RunInput, Fail> {
    must_exist(&args.workload)?;
    must_exist(&args.targets)?;
    must_exist(&args.requests)?;
    crate::schema::validate_workload(&args.workload)?;
    crate::schema::validate_targets(&args.targets)?;

    let workload = load_json::<Workload>(&args.workload)?;
    validate_workload(&workload)?;
    let suite = args.suite.as_str();
    if workload.suite != suite {
        return Err(Fail::new(
            Code::InvalidInput,
            format!(
                "workload suite mismatch: cli={}, file={}",
                suite, workload.suite
            ),
        ));
    }

    let targets = load_json::<Vec<Target>>(&args.targets)?;
    if targets.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            "targets list must not be empty",
        ));
    }
    validate_targets(&targets)?;

    let request_value = load_json::<serde_json::Value>(&args.requests)?;
    let requests = parse_requests(request_value)?;
    let request_count_hint = requests.len();
    let request_skip = workload.requests.skip.clone();

    let trials = args.trials.unwrap_or_else(|| args.class.default_trials());
    let limits = workload.limits;
    Ok(RunInput {
        suite,
        seed: workload.data.seed,
        trials,
        git: git_sha_short(),
        workload_hash: sha256_file(&args.workload)?,
        targets,
        requests,
        request_skip,
        request_count_hint,
        limits,
    })
}

fn parse_requests(value: serde_json::Value) -> Result<Vec<RequestDoc>, Fail> {
    let Some(items) = value.as_array() else {
        return Err(Fail::new(
            Code::InvalidInput,
            "requests file must be a JSON array at the top level",
        ));
    };

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match item {
            serde_json::Value::String(path) => out.push(RequestDoc {
                method: "GET".to_string(),
                path: path.clone(),
                query: BTreeMap::new(),
            }),
            serde_json::Value::Object(map) => {
                let method = map
                    .get("method")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("GET")
                    .to_string();
                let path = map
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("/customers")
                    .to_string();
                let mut query = BTreeMap::new();
                if let Some(obj) = map.get("query").and_then(serde_json::Value::as_object) {
                    for (key, val) in obj {
                        query.insert(
                            key.clone(),
                            val.as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| val.to_string()),
                        );
                    }
                }
                out.push(RequestDoc {
                    method,
                    path,
                    query,
                });
            }
            _ => {
                return Err(Fail::new(
                    Code::InvalidInput,
                    "request entry must be string path or object",
                ));
            }
        }
    }
    Ok(out)
}

fn materialize_requests(
    run_dir: &Path,
    seed: u64,
    input: Vec<RequestDoc>,
    skip: &[String],
    hint: usize,
) -> Result<Vec<RequestDoc>, Fail> {
    let mut rng = StdRng::seed_from_u64(seed);

    // Dataset sizes (micro) matching drizzle-benchmarks/src/seed.ts
    let n_customers = crate::load::SEED_CUSTOMERS; // 10_000
    let n_employees = crate::load::SEED_EMPLOYEES; // 200
    let n_suppliers = crate::load::SEED_SUPPLIERS; // 1_000
    let n_products = crate::load::SEED_PRODUCTS; // 5_000
    let n_orders = crate::load::SEED_ORDERS; // 50_000

    let mut out = if input.is_empty() {
        let mut pool = Vec::with_capacity(hint.max(query_catalog_total_mix()));

        // Helper to make a GET request
        let req = |path: &str, query: BTreeMap<String, String>| RequestDoc {
            method: "GET".to_string(),
            path: path.to_string(),
            query,
        };

        // Mirrors drizzle-benchmarks/src/generate.ts: for (let i = 1; i < 2e4; i += 1)
        for i in 0..MIX_CUSTOMER_BY_ID {
            let id = (i % n_customers) + 1;
            let mut q = BTreeMap::new();
            q.insert("id".to_string(), id.to_string());
            pool.push(req("/customer-by-id", q));
        }

        // 5k employee-with-recipient (IDs 1..=n_employees)
        for i in 0..MIX_EMPLOYEE_WITH_RECIPIENT {
            let id = (i % n_employees) + 1;
            let mut q = BTreeMap::new();
            q.insert("id".to_string(), id.to_string());
            pool.push(req("/employee-with-recipient", q));
        }

        // 30k supplier-by-id (IDs 1..=n_suppliers)
        for i in 0..MIX_SUPPLIER_BY_ID {
            let id = (i % n_suppliers) + 1;
            let mut q = BTreeMap::new();
            q.insert("id".to_string(), id.to_string());
            pool.push(req("/supplier-by-id", q));
        }

        // 100k product-with-supplier (IDs 1..=n_products)
        for i in 0..MIX_PRODUCT_WITH_SUPPLIER {
            let id = (i % n_products) + 1;
            let mut q = BTreeMap::new();
            q.insert("id".to_string(), id.to_string());
            pool.push(req("/product-with-supplier", q));
        }

        // 100k order-with-details (IDs 1..=n_orders)
        for i in 0..MIX_ORDER_WITH_DETAILS {
            let id = (i % n_orders) + 1;
            let mut q = BTreeMap::new();
            q.insert("id".to_string(), id.to_string());
            pool.push(req("/order-with-details", q));
        }

        // 100k order-with-details-and-products (IDs 1..=n_orders)
        for i in 0..MIX_ORDER_WITH_DETAILS_AND_PRODUCTS {
            let id = (i % n_orders) + 1;
            let mut q = BTreeMap::new();
            q.insert("id".to_string(), id.to_string());
            pool.push(req("/order-with-details-and-products", q));
        }

        // 2k paginated customers (limit=50, random pages)
        for _ in 0..MIX_CUSTOMERS {
            let pages = n_customers / 50;
            let page = 1 + (rng.random_range(0..pages as u64) as usize);
            let offset = page * 50 - 50;
            let mut q = BTreeMap::new();
            q.insert("limit".to_string(), "50".to_string());
            q.insert("offset".to_string(), offset.to_string());
            pool.push(req("/customers", q));
        }

        // 1k paginated employees (limit=20, random pages)
        for _ in 0..MIX_EMPLOYEES {
            let pages = (n_employees / 20).max(1);
            let page = 1 + (rng.random_range(0..pages as u64) as usize);
            let offset = page * 20 - 20;
            let mut q = BTreeMap::new();
            q.insert("limit".to_string(), "20".to_string());
            q.insert("offset".to_string(), offset.to_string());
            pool.push(req("/employees", q));
        }

        // 1k paginated suppliers (limit=50, random pages)
        for _ in 0..MIX_SUPPLIERS {
            let pages = (n_suppliers / 50).max(1);
            let page = 1 + (rng.random_range(0..pages as u64) as usize);
            let offset = page * 50 - 50;
            let mut q = BTreeMap::new();
            q.insert("limit".to_string(), "50".to_string());
            q.insert("offset".to_string(), offset.to_string());
            pool.push(req("/suppliers", q));
        }

        // 3k paginated products (limit=50, random pages)
        for _ in 0..MIX_PRODUCTS {
            let pages = (n_products / 50).max(1);
            let page = 1 + (rng.random_range(0..pages as u64) as usize);
            let offset = page * 50 - 50;
            let mut q = BTreeMap::new();
            q.insert("limit".to_string(), "50".to_string());
            q.insert("offset".to_string(), offset.to_string());
            pool.push(req("/products", q));
        }

        // 10k paginated orders-with-details (limit=50, random pages)
        for _ in 0..MIX_ORDERS_WITH_DETAILS {
            let pages = (n_orders / 50).max(1);
            let page = 1 + (rng.random_range(0..pages as u64) as usize);
            let offset = page * 50 - 50;
            let mut q = BTreeMap::new();
            q.insert("limit".to_string(), "50".to_string());
            q.insert("offset".to_string(), offset.to_string());
            pool.push(req("/orders-with-details", q));
        }

        // 5k search-customer requests
        for i in 0..MIX_SEARCH_CUSTOMER {
            let term = CUSTOMER_SEARCH_TERMS[i % CUSTOMER_SEARCH_TERMS.len()];
            let mut q = BTreeMap::new();
            q.insert("term".to_string(), term.to_string());
            pool.push(req("/search-customer", q));
        }

        // 50k search-product requests (same search terms)
        for i in 0..MIX_SEARCH_PRODUCT {
            let term = PRODUCT_SEARCH_TERMS[i % PRODUCT_SEARCH_TERMS.len()];
            let mut q = BTreeMap::new();
            q.insert("term".to_string(), term.to_string());
            pool.push(req("/search-product", q));
        }

        // Shuffle deterministically
        use rand::seq::SliceRandom;
        pool.shuffle(&mut rng);
        pool
    } else {
        input
    };

    for req in &mut out {
        if req.path.is_empty() {
            req.path = "/customers".to_string();
        }
        if req.method.is_empty() {
            req.method = "GET".to_string();
        }
    }

    if !skip.is_empty() {
        out.retain(|req| !request_path_skipped(&req.path, skip));
    }

    for (idx, req) in out.iter_mut().enumerate() {
        req.query
            .entry("seed".to_string())
            .or_insert_with(|| seed.to_string());
        req.query
            .entry("idx".to_string())
            .or_insert_with(|| idx.to_string());
    }

    let path = run_dir.join("requests.generated.json");
    write_json(path, &out, Code::RunFail)?;
    Ok(out)
}

fn request_path_skipped(path: &str, skip: &[String]) -> bool {
    skip.iter().any(|prefix| path.starts_with(prefix.as_str()))
}

fn init_layout(run_dir: &Path, targets: &[Target]) -> Result<(), Fail> {
    fs::create_dir_all(run_dir).map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("failed to create run dir {}: {e}", run_dir.display()),
        )
    })?;
    fs::create_dir_all(run_dir.join("reports")).map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("failed to create reports dir in {}: {e}", run_dir.display()),
        )
    })?;
    for target in targets {
        let base = run_dir.join("targets").join(&target.id);
        fs::create_dir_all(base.join("raw")).map_err(|e| {
            Fail::new(
                Code::RunFail,
                format!("failed to create target dir {}: {e}", base.display()),
            )
        })?;
    }
    Ok(())
}

fn write_env(
    run_dir: &Path,
    args: &Run,
    suite: &str,
    trials: u32,
    requests: usize,
    run_id: &str,
    cohort_id: &str,
) -> Result<(), Fail> {
    let env = serde_json::json!({
        "version": "v1",
        "run_id": run_id,
        "cohort_id": cohort_id,
        "cohort_arg": args.cohort_id,
        "suite": suite,
        "class": class_name(args.class),
        "trials": trials,
        "requests": requests,
        "publish": args.publish,
        "seed": args.seed,
        "timeout_s": args.timeout_s,
        "workload": args.workload,
        "targets": args.targets,
        "requests_file": args.requests,
        "requests_generated": "requests.generated.json",
        "baseline": args.baseline,
    });
    write_json(run_dir.join("env.json"), &env, Code::RunFail)
}

fn run_parity(
    targets: &[Target],
    seed: u64,
    seed_file: &Path,
    events: &mut BufWriter<File>,
    json: bool,
) -> Result<(), Fail> {
    emit(events, json, "info", "parity", "start")?;
    for target in targets {
        let mut env = BTreeMap::new();
        env.insert("BENCH_TARGET_ID".to_string(), target.id.clone());
        env.insert("BENCH_SEED".to_string(), seed.to_string());
        env.insert("BENCH_TRIAL".to_string(), "1".to_string());
        env.insert(
            "BENCH_SEED_FILE".to_string(),
            seed_file.to_string_lossy().to_string(),
        );
        add_server_env(target, &mut env);
        exec_cmd(&target.parity, &target.id, "parity", Code::ParityFail, &env)?;
        emit(
            events,
            json,
            "info",
            "parity",
            format!("target={} pass", target.id),
        )?;
    }
    Ok(())
}

fn run_warmup(targets: &[Target], events: &mut BufWriter<File>, json: bool) -> Result<(), Fail> {
    emit(events, json, "info", "warmup", "start")?;
    for target in targets {
        run_target_exec(target, target.warmup.as_ref(), "warmup", Code::RunFail)?;
    }
    emit(events, json, "info", "warmup", "done")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_trials(
    run_dir: &Path,
    suite: &str,
    targets: &[Target],
    events: &mut BufWriter<File>,
    json: bool,
    trials: u32,
    seed: u64,
    seed_file: &Path,
    workload_path: &Path,
    requests_path: &Path,
) -> Result<BTreeMap<String, Vec<TrialMeasurement>>, Fail> {
    emit(
        events,
        json,
        "info",
        "trials",
        format!("start count={trials}"),
    )?;

    let mut measurements: BTreeMap<String, Vec<TrialMeasurement>> = targets
        .iter()
        .map(|target| (target.id.clone(), Vec::new()))
        .collect();

    for trial in 0..trials {
        let mut order: Vec<(usize, &Target)> = targets.iter().enumerate().collect();
        if !order.is_empty() {
            let shift = trial as usize % order.len();
            order.rotate_left(shift);
        }
        let target_order = order
            .iter()
            .map(|(_, target)| target.id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        emit(
            events,
            json,
            "info",
            "trial",
            format!("run index={} target_order={target_order}", trial + 1),
        )?;
        for (idx, target) in order {
            let measurement = run_target_load(
                run_dir,
                suite,
                target,
                trial,
                seed,
                idx,
                seed_file,
                workload_path,
                requests_path,
            )?;
            measurements
                .entry(target.id.clone())
                .or_default()
                .push(measurement);
        }
    }
    emit(events, json, "info", "trials", "done")?;
    Ok(measurements)
}

fn run_aggregate(
    run_dir: &Path,
    run_id: &str,
    input: &RunInput,
    measurements: &BTreeMap<String, Vec<TrialMeasurement>>,
    events: &mut BufWriter<File>,
    json: bool,
) -> Result<Aggregate, Fail> {
    emit(events, json, "info", "aggregate", "start")?;

    let mut summary_map: BTreeMap<String, PrimaryDoc> = BTreeMap::new();
    for target in &input.targets {
        let values = measurements
            .get(&target.id)
            .ok_or_else(|| Fail::new(Code::AggregateFail, "missing target trial points"))?;
        let summary = compute_primary(values);
        let spread = compute_spread(values, input.trials);
        let saturation_points = combined_series(values);
        let saturation = compute_saturation(&saturation_points);
        summary_map.insert(target.id.clone(), summary.clone());
        write_target_artifacts(
            run_dir,
            TargetArtifacts {
                run_id,
                suite: input.suite,
                target_id: &target.id,
                group: target.group.as_deref(),
                measurements: values,
                summary: &summary,
                spread: &spread,
                saturation: &saturation,
            },
        )?;
    }

    let headroom = compute_headroom(measurements);

    emit(events, json, "info", "aggregate", "done")?;
    Ok(Aggregate {
        headroom,
        summaries: summary_map,
    })
}

fn write_target_artifacts(run_dir: &Path, doc: TargetArtifacts<'_>) -> Result<(), Fail> {
    let target_dir = run_dir.join("targets").join(doc.target_id);
    let raw_dir = target_dir.join("raw");
    let points = combined_series(doc.measurements);

    write_k6_csv(raw_dir.join("k6.csv"), &points)?;
    write_cpu_csv(raw_dir.join("cpu.csv"), &points)?;
    write_k6_parquet(raw_dir.join("k6.parquet"), &points)?;

    let summary_doc = SummaryDoc {
        version: "v1",
        run_id: doc.run_id.to_string(),
        suite: doc.suite.to_string(),
        target_id: doc.target_id.to_string(),
        group: doc.group.map(str::to_string),
        primary: doc.summary.clone(),
        spread: doc.spread.clone(),
        saturation: doc.saturation.clone(),
    };
    write_json(
        target_dir.join("summary.json"),
        &summary_doc,
        Code::AggregateFail,
    )?;
    crate::schema::validate_summary(&target_dir.join("summary.json"))?;

    let series = TimeseriesDoc {
        version: "v1",
        run_id: doc.run_id.to_string(),
        suite: doc.suite.to_string(),
        target_id: doc.target_id.to_string(),
        interval_s: 1,
        points,
    };
    write_json(
        target_dir.join("timeseries.json"),
        &series,
        Code::AggregateFail,
    )?;
    crate::schema::validate_timeseries(&target_dir.join("timeseries.json"))?;
    Ok(())
}

fn resolve_baseline(
    baseline_id: Option<&str>,
    out: &Path,
    suite: &str,
) -> Result<Option<Baseline>, Fail> {
    let run_id = match baseline_id {
        Some("auto") => match find_latest_run(out, suite) {
            Some(id) => id,
            None => return Ok(None),
        },
        Some(id) => id.to_string(),
        None => return Ok(None),
    };

    let run_dir = out.join("runs").join(&run_id);
    if !run_dir.exists() {
        return Err(Fail::new(
            Code::NoBaseline,
            format!("baseline run not found: {run_id}"),
        ));
    }

    let manifest_path = run_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Err(Fail::new(
            Code::NoBaseline,
            format!("baseline manifest missing: {}", manifest_path.display()),
        ));
    }

    #[derive(serde::Deserialize)]
    struct ManifestIn {
        suite: String,
    }
    let manifest = load_json::<ManifestIn>(&manifest_path)?;
    if manifest.suite != suite {
        return Err(Fail::new(
            Code::NoBaseline,
            format!(
                "baseline suite mismatch: expected {}, found {}",
                suite, manifest.suite
            ),
        ));
    }

    let targets_dir = run_dir.join("targets");
    if !targets_dir.exists() {
        return Err(Fail::new(
            Code::NoBaseline,
            format!("baseline targets missing: {}", targets_dir.display()),
        ));
    }

    let mut summaries = BTreeMap::new();
    for entry in fs::read_dir(&targets_dir).map_err(|e| {
        Fail::new(
            Code::NoBaseline,
            format!(
                "failed to read baseline targets {}: {e}",
                targets_dir.display()
            ),
        )
    })? {
        let entry = entry
            .map_err(|e| Fail::new(Code::NoBaseline, format!("baseline dir entry error: {e}")))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let summary_path = path.join("summary.json");
        if !summary_path.exists() {
            continue;
        }
        #[derive(serde::Deserialize)]
        struct BaselineSummary {
            target_id: String,
            primary: PrimaryDoc,
        }
        let summary = load_json::<BaselineSummary>(&summary_path)?;
        summaries.insert(summary.target_id, summary.primary);
    }

    if summaries.is_empty() {
        return Err(Fail::new(
            Code::NoBaseline,
            format!("baseline has no target summaries: {}", run_id),
        ));
    }

    Ok(Some(Baseline {
        run_id: run_id.to_string(),
        summaries,
    }))
}

fn find_latest_run(out: &Path, suite: &str) -> Option<String> {
    let runs_dir = out.join("runs");
    if !runs_dir.is_dir() {
        return None;
    }
    let Ok(entries) = fs::read_dir(&runs_dir) else {
        return None;
    };

    #[derive(serde::Deserialize)]
    struct ManifestPeek {
        suite: String,
        status: String,
    }

    let mut candidates: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let Ok(body) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<ManifestPeek>(&body) else {
            continue;
        };
        if manifest.suite == suite
            && manifest.status == "success"
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            candidates.push(name.to_string());
        }
    }

    // Run IDs are YYYYMMDDTHHMMSSZ_git_suite — lexicographic sort gives chronological order
    candidates.sort();
    candidates.pop()
}

fn run_gates(
    class: Class,
    headroom: &Headroom,
    current: &BTreeMap<String, PrimaryDoc>,
    baseline: Option<&Baseline>,
    limits: &Limits,
    events: &mut BufWriter<File>,
    json: bool,
) -> Result<Gates, Fail> {
    let headroom_ok = headroom.cpu_peak < 85.0;
    let headroom_gate = if headroom_ok { Gate::Pass } else { Gate::Fail };
    let regression_gate = regression_gate(current, baseline);
    let limits_gate = limits_gate(current, limits);
    let gates = Gates {
        parity: Gate::Pass,
        headroom: headroom_gate,
        regression: regression_gate,
        limits: limits_gate,
    };

    emit(
        events,
        json,
        "info",
        "gates",
        format!(
            "headroom cpu_peak={:.2} net_peak={} status={}",
            headroom.cpu_peak,
            headroom
                .net_peak
                .map(|peak| format!("{peak:.2}"))
                .unwrap_or_else(|| "unmeasured".to_string()),
            if headroom_ok { "pass" } else { "fail" }
        ),
    )?;
    emit(
        events,
        json,
        "info",
        "gates",
        format!("regression status={}", gate_name(&gates.regression)),
    )?;
    emit(
        events,
        json,
        "info",
        "gates",
        format!("limits status={}", gate_name(&gates.limits)),
    )?;

    if !headroom_ok && matches!(class, Class::Publish) {
        return Err(Fail::new(
            Code::GateFail,
            "headroom gate failed for publish run",
        ));
    }
    if matches!(gates.regression, Gate::Fail) && matches!(class, Class::Publish) {
        return Err(Fail::new(
            Code::GateFail,
            "regression gate failed for publish run",
        ));
    }
    if matches!(gates.limits, Gate::Fail) && matches!(class, Class::Publish) {
        return Err(Fail::new(
            Code::GateFail,
            "limits gate failed for publish run",
        ));
    }
    Ok(gates)
}

fn run_publish(events: &mut BufWriter<File>, json: bool) -> Result<(), Fail> {
    emit(events, json, "info", "publish", "start")?;
    emit(
        events,
        json,
        "info",
        "publish",
        "done (artifact stage only)",
    )?;
    Ok(())
}

fn run_target_exec(
    target: &Target,
    spec: Option<&Exec>,
    step: &'static str,
    code: Code,
) -> Result<(), Fail> {
    let Some(spec) = spec else {
        return Ok(());
    };
    exec_cmd(spec, &target.id, step, code, &BTreeMap::new())
}

#[allow(clippy::too_many_arguments)]
fn run_target_load(
    run_dir: &Path,
    suite: &str,
    target: &Target,
    trial: u32,
    seed: u64,
    idx: usize,
    seed_file: &Path,
    workload_path: &Path,
    requests_path: &Path,
) -> Result<TrialMeasurement, Fail> {
    let spec = &target.load;

    let scratch = run_dir.join(".tmp");
    fs::create_dir_all(&scratch).map_err(|err| {
        Fail::new(
            Code::RunFail,
            format!("failed to create {}: {err}", scratch.display()),
        )
    })?;
    let point_path = scratch.join(format!(
        "{}-{}-{}.json",
        sanitize(&target.id),
        trial + 1,
        idx + 1
    ));
    let series_path = scratch.join(format!(
        "{}-{}-{}.series.json",
        sanitize(&target.id),
        trial + 1,
        idx + 1
    ));
    let _ = fs::remove_file(&point_path);
    let _ = fs::remove_file(&series_path);

    let mut env = BTreeMap::new();
    env.insert(
        "BENCH_POINT_OUT".to_string(),
        point_path.to_string_lossy().to_string(),
    );
    env.insert(
        "BENCH_TIMESERIES_OUT".to_string(),
        series_path.to_string_lossy().to_string(),
    );
    env.insert("BENCH_RUN_DIR".to_string(), run_dir.display().to_string());
    env.insert("BENCH_SUITE".to_string(), suite.to_string());
    env.insert("BENCH_TARGET_ID".to_string(), target.id.clone());
    env.insert("BENCH_TRIAL".to_string(), trial.to_string());
    env.insert("BENCH_SEED".to_string(), seed.to_string());
    env.insert(
        "BENCH_SEED_FILE".to_string(),
        seed_file.to_string_lossy().to_string(),
    );
    env.insert(
        "BENCH_WORKLOAD_FILE".to_string(),
        workload_path.to_string_lossy().to_string(),
    );
    env.insert(
        "BENCH_REQUESTS_FILE".to_string(),
        requests_path.to_string_lossy().to_string(),
    );
    add_server_env(target, &mut env);

    exec_cmd(spec, &target.id, "load", Code::RunFail, &env)?;

    let raw_dir = run_dir
        .join("targets")
        .join(&target.id)
        .join("raw")
        .join("trial");
    fs::create_dir_all(&raw_dir).map_err(|err| {
        Fail::new(
            Code::RunFail,
            format!("failed to create {}: {err}", raw_dir.display()),
        )
    })?;

    let measurement = if series_path.exists() {
        let series = load_points(&series_path)?;
        if series.is_empty() {
            return Err(Fail::new(
                Code::RunFail,
                format!("target {} load emitted empty series", target.id),
            ));
        }
        for point in &series {
            validate_point(&target.id, point)?;
        }
        let trial_path = raw_dir.join(format!("{}.series.json", trial + 1));
        fs::copy(&series_path, &trial_path).map_err(|err| {
            Fail::new(
                Code::RunFail,
                format!(
                    "failed to copy {} to {}: {err}",
                    series_path.display(),
                    trial_path.display()
                ),
            )
        })?;
        TrialMeasurement {
            aggregate: point_from_series(&series),
            series,
        }
    } else if point_path.exists() {
        let trial_path = raw_dir.join(format!("{}.point.json", trial + 1));
        fs::copy(&point_path, &trial_path).map_err(|err| {
            Fail::new(
                Code::RunFail,
                format!(
                    "failed to copy {} to {}: {err}",
                    point_path.display(),
                    trial_path.display()
                ),
            )
        })?;
        let point = load_json_with_code::<Point>(&point_path, Code::RunFail)?;
        TrialMeasurement {
            aggregate: point.clone(),
            series: vec![point],
        }
    } else {
        return Err(Fail::new(
            Code::RunFail,
            format!(
                "target {} load emitted neither point nor series artifact",
                target.id
            ),
        ));
    };
    let _ = fs::remove_file(&point_path);
    let _ = fs::remove_file(&series_path);
    validate_point(&target.id, &measurement.aggregate)?;
    Ok(measurement)
}

fn add_server_env(target: &Target, env: &mut BTreeMap<String, String>) {
    env.insert("BENCH_POOL_SIZE".to_string(), target.pool.max.to_string());
    if let Some(server) = &target.server {
        let cmd_json = serde_json::to_string(&server.cmd).unwrap_or_default();
        env.insert("BENCH_SERVER_CMD".to_string(), cmd_json);
        if let Some(cwd) = &server.cwd {
            env.insert("BENCH_SERVER_CWD".to_string(), cwd.clone());
        }
        for (k, v) in &server.env {
            env.insert(k.clone(), v.clone());
        }
    }
}

fn exec_cmd(
    spec: &Exec,
    target_id: &str,
    step: &'static str,
    code: Code,
    extra: &BTreeMap<String, String>,
) -> Result<(), Fail> {
    if spec.cmd.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("target {target_id} {step} command is empty"),
        ));
    }

    let mut cmd = Command::new(&spec.cmd[0]);
    cmd.args(&spec.cmd[1..]);
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }
    cmd.envs(&spec.env);
    cmd.envs(extra);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    if let Some(timeout_s) = spec.timeout_s {
        let mut child = cmd
            .spawn()
            .map_err(|e| Fail::new(code, format!("target {target_id} {step} spawn failed: {e}")))?;
        let deadline = Instant::now() + Duration::from_secs(timeout_s);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        return Ok(());
                    }
                    return Err(Fail::new(
                        code,
                        format!(
                            "target {target_id} {step} failed with code {:?}",
                            status.code()
                        ),
                    ));
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(Fail::new(
                            code,
                            format!("target {target_id} {step} timed out"),
                        ));
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(err) => {
                    return Err(Fail::new(
                        code,
                        format!("target {target_id} {step} wait failed: {err}"),
                    ));
                }
            }
        }
    }

    let status = cmd.status().map_err(|e| {
        Fail::new(
            code,
            format!("target {target_id} {step} failed to start: {e}"),
        )
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(Fail::new(
            code,
            format!(
                "target {target_id} {step} failed with code {:?}",
                status.code()
            ),
        ))
    }
}

fn validate_point(target_id: &str, point: &Point) -> Result<(), Fail> {
    if point.time.is_empty() {
        return Err(Fail::new(
            Code::RunFail,
            format!("target {target_id} load point is missing time"),
        ));
    }
    if point.rps < 0.0 || !(0.0..=1.0).contains(&point.err) {
        return Err(Fail::new(
            Code::RunFail,
            format!("target {target_id} load point has invalid rps or err"),
        ));
    }
    if point.cpu.is_empty() || point.cpu.iter().any(|cpu| !(0.0..=100.0).contains(cpu)) {
        return Err(Fail::new(
            Code::RunFail,
            format!("target {target_id} load point has invalid cpu values"),
        ));
    }
    if point.latency.avg < 0.0
        || point.latency.p95 < 0.0
        || point.latency.p99 < 0.0
        || point.latency.p999.is_some_and(|value| value < 0.0)
    {
        return Err(Fail::new(
            Code::RunFail,
            format!("target {target_id} load point has invalid latency values"),
        ));
    }
    for query in &point.queries {
        if query.method.is_empty()
            || query.path.is_empty()
            || query.rps < 0.0
            || !(0.0..=1.0).contains(&query.err)
            || query.latency.avg < 0.0
            || query.latency.p95 < 0.0
            || query.latency.p99 < 0.0
            || query.latency.p999.is_some_and(|value| value < 0.0)
        {
            return Err(Fail::new(
                Code::RunFail,
                format!("target {target_id} load point has invalid query metric values"),
            ));
        }
    }
    Ok(())
}

fn load_points(path: &Path) -> Result<Vec<Point>, Fail> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum LoadSeries {
        Points(Vec<Point>),
        Doc { points: Vec<Point> },
    }

    match load_json_with_code::<LoadSeries>(path, Code::RunFail)? {
        LoadSeries::Points(points) => Ok(points),
        LoadSeries::Doc { points } => Ok(points),
    }
}

fn point_from_series(points: &[Point]) -> Point {
    let time = points
        .last()
        .map(|point| point.time.clone())
        .unwrap_or_else(now_tag);
    let rps: Vec<f64> = points.iter().map(|point| point.rps).collect();
    let err: Vec<f64> = points.iter().map(|point| point.err).collect();
    let lat_avg: Vec<f64> = points.iter().map(|point| point.latency.avg).collect();
    let lat_p95: Vec<f64> = points.iter().map(|point| point.latency.p95).collect();
    let lat_p99: Vec<f64> = points.iter().map(|point| point.latency.p99).collect();
    let lat_p999: Vec<f64> = points
        .iter()
        .map(|point| point.latency.p999.unwrap_or(point.latency.p99))
        .collect();
    let cpu: Vec<f64> = points.iter().map(|point| avg(&point.cpu)).collect();

    let mem: Vec<f64> = points.iter().filter_map(|p| p.mem_mb).collect();
    Point {
        time,
        rps: avg(&rps),
        err: avg(&err),
        latency: crate::model::Latency {
            avg: avg(&lat_avg),
            p95: median(&lat_p95),
            p99: median(&lat_p99),
            p999: Some(median(&lat_p999)),
        },
        cpu: if cpu.is_empty() { vec![0.0] } else { cpu },
        mem_mb: if mem.is_empty() {
            None
        } else {
            Some(avg(&mem))
        },
        queries: Vec::new(),
    }
}

fn combined_series(measurements: &[TrialMeasurement]) -> Vec<Point> {
    measurements
        .iter()
        .flat_map(|measurement| measurement.series.iter().cloned())
        .collect()
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn write_k6_csv(path: PathBuf, points: &[Point]) -> Result<(), Fail> {
    let mut body = String::from("time,rps,err,latency_avg,latency_p95,latency_p99\n");
    for point in points {
        body.push_str(&format!(
            "{},{:.3},{:.6},{:.3},{:.3},{:.3}\n",
            point.time,
            point.rps,
            point.err,
            point.latency.avg,
            point.latency.p95,
            point.latency.p99
        ));
    }
    fs::write(&path, body).map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("write {} failed: {e}", path.display()),
        )
    })
}

fn write_cpu_csv(path: PathBuf, points: &[Point]) -> Result<(), Fail> {
    let mut body = String::from("time,cpu_avg,cpu_peak\n");
    for point in points {
        let avg = avg(&point.cpu);
        let peak = peak(&point.cpu);
        body.push_str(&format!("{},{:.3},{:.3}\n", point.time, avg, peak));
    }
    fs::write(&path, body).map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("write {} failed: {e}", path.display()),
        )
    })
}

fn write_k6_parquet(path: PathBuf, points: &[Point]) -> Result<(), Fail> {
    let schema = Arc::new(
        parse_message_type(
            "
            message k6 {
              REQUIRED BINARY time (STRING);
              REQUIRED DOUBLE rps;
              REQUIRED DOUBLE err;
              REQUIRED DOUBLE lat_avg;
              REQUIRED DOUBLE lat_p95;
              REQUIRED DOUBLE lat_p99;
              REQUIRED DOUBLE lat_p999;
              REQUIRED DOUBLE cpu_avg;
              REQUIRED DOUBLE cpu_peak;
            }
            ",
        )
        .map_err(|e| Fail::new(Code::AggregateFail, format!("parquet schema failed: {e}")))?,
    );
    let file = File::create(&path).map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("create {} failed: {e}", path.display()),
        )
    })?;
    let props = Arc::new(WriterProperties::builder().build());
    let mut writer = SerializedFileWriter::new(file, schema, props).map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("open parquet {} failed: {e}", path.display()),
        )
    })?;

    let time: Vec<ByteArray> = points
        .iter()
        .map(|point| ByteArray::from(point.time.as_str()))
        .collect();
    let rps: Vec<f64> = points.iter().map(|point| point.rps).collect();
    let err: Vec<f64> = points.iter().map(|point| point.err).collect();
    let lat_avg: Vec<f64> = points.iter().map(|point| point.latency.avg).collect();
    let lat_p95: Vec<f64> = points.iter().map(|point| point.latency.p95).collect();
    let lat_p99: Vec<f64> = points.iter().map(|point| point.latency.p99).collect();
    let lat_p999: Vec<f64> = points
        .iter()
        .map(|point| point.latency.p999.unwrap_or(point.latency.p99))
        .collect();
    let cpu_avg: Vec<f64> = points.iter().map(|point| avg(&point.cpu)).collect();
    let cpu_peak: Vec<f64> = points.iter().map(|point| peak(&point.cpu)).collect();

    let mut row_group = writer.next_row_group().map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("start parquet row group {} failed: {e}", path.display()),
        )
    })?;
    let mut idx = 0_usize;
    while let Some(mut col) = row_group.next_column().map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("next parquet column {} failed: {e}", path.display()),
        )
    })? {
        match idx {
            0 => col.typed::<ByteArrayType>().write_batch(&time, None, None),
            1 => col.typed::<DoubleType>().write_batch(&rps, None, None),
            2 => col.typed::<DoubleType>().write_batch(&err, None, None),
            3 => col.typed::<DoubleType>().write_batch(&lat_avg, None, None),
            4 => col.typed::<DoubleType>().write_batch(&lat_p95, None, None),
            5 => col.typed::<DoubleType>().write_batch(&lat_p99, None, None),
            6 => col.typed::<DoubleType>().write_batch(&lat_p999, None, None),
            7 => col.typed::<DoubleType>().write_batch(&cpu_avg, None, None),
            8 => col.typed::<DoubleType>().write_batch(&cpu_peak, None, None),
            other => {
                return Err(Fail::new(
                    Code::AggregateFail,
                    format!(
                        "unexpected parquet column index {other} for {}",
                        path.display()
                    ),
                ));
            }
        }
        .map_err(|e| {
            Fail::new(
                Code::AggregateFail,
                format!("write parquet column {} failed: {e}", path.display()),
            )
        })?;
        col.close().map_err(|e| {
            Fail::new(
                Code::AggregateFail,
                format!("close parquet column {} failed: {e}", path.display()),
            )
        })?;
        idx += 1;
    }
    row_group.close().map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("close parquet row group {} failed: {e}", path.display()),
        )
    })?;
    writer.close().map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("close parquet {} failed: {e}", path.display()),
        )
    })?;
    Ok(())
}

fn write_compare_report(
    run_dir: &Path,
    summaries: &BTreeMap<String, PrimaryDoc>,
    baseline: Option<&Baseline>,
) -> Result<(), Fail> {
    let mut body = String::from(
        "| target | rps.avg | latency.p95 | err | base.rps | delta.rps | base.p95 | delta.p95 |\n",
    );
    body.push_str("|---|---:|---:|---:|---:|---:|---:|---:|\n");
    for (id, sum) in summaries {
        let (base_rps, delta_rps, base_p95, delta_p95) = if let Some(base) =
            baseline.and_then(|b| b.summaries.get(id)).map(|p| {
                (
                    p.rps.avg,
                    sum.rps.avg - p.rps.avg,
                    p.latency.p95,
                    sum.latency.p95 - p.latency.p95,
                )
            }) {
            (
                format!("{:.2}", base.0),
                format!("{:+.2}", base.1),
                format!("{:.2}", base.2),
                format!("{:+.2}", base.3),
            )
        } else {
            (
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
            )
        };

        body.push_str(&format!(
            "| {} | {:.2} | {:.2} | {:.5} | {} | {} | {} | {} |\n",
            id, sum.rps.avg, sum.latency.p95, sum.err, base_rps, delta_rps, base_p95, delta_p95
        ));
    }
    if let Some(base) = baseline {
        body.push_str(&format!("\nBaseline: `{}`\n", base.run_id));
    }
    fs::write(run_dir.join("reports").join("compare.md"), body).map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("failed writing compare report: {e}"),
        )
    })
}

fn run_name(suite: &str, class: Class) -> String {
    format!("{} ({})", suite_label(suite), class_name(class))
}

fn suite_label(suite: &str) -> String {
    match suite {
        "throughput-http" => "Throughput HTTP".to_string(),
        "mvcc-contention" => "MVCC Contention".to_string(),
        other => humanize_slug(other),
    }
}

fn humanize_slug(value: &str) -> String {
    value
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn target_meta_doc(target: &Target) -> TargetMetaDoc {
    TargetMetaDoc {
        id: target.id.clone(),
        name: target.display.name.clone(),
        description: target.display.description.clone(),
        group: target.group.clone(),
        lang: target.lang.clone(),
        runtime: target.runtime.clone(),
        orm: target.orm.clone(),
        driver: target.driver.clone(),
        proc: target.proc.clone(),
        pool: target.pool.clone(),
        db: target.db.clone(),
        wire: target.wire.clone(),
        fair: target.fair.clone(),
        contract: target.contract.clone(),
    }
}

fn query_catalog_total_mix() -> usize {
    [
        MIX_CUSTOMER_BY_ID,
        MIX_EMPLOYEE_WITH_RECIPIENT,
        MIX_SUPPLIER_BY_ID,
        MIX_PRODUCT_WITH_SUPPLIER,
        MIX_ORDER_WITH_DETAILS,
        MIX_ORDER_WITH_DETAILS_AND_PRODUCTS,
        MIX_CUSTOMERS,
        MIX_EMPLOYEES,
        MIX_SUPPLIERS,
        MIX_PRODUCTS,
        MIX_ORDERS_WITH_DETAILS,
        MIX_SEARCH_CUSTOMER,
        MIX_SEARCH_PRODUCT,
    ]
    .iter()
    .sum()
}

fn query_catalog() -> Vec<QueryDoc> {
    vec![
        query_doc(
            "customer-by-id",
            "Customer by id",
            "/customer-by-id",
            MIX_CUSTOMER_BY_ID,
            &["id"],
            &[
                "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE id = :id",
            ],
        ),
        query_doc(
            "employee-with-recipient",
            "Employee with manager",
            "/employee-with-recipient",
            MIX_EMPLOYEE_WITH_RECIPIENT,
            &["id"],
            &[
                "SELECT e.*, r.last_name AS recipient_last_name, r.first_name AS recipient_first_name FROM employees e LEFT JOIN employees r ON e.recipient_id = r.id WHERE e.id = :id",
            ],
        ),
        query_doc(
            "supplier-by-id",
            "Supplier by id",
            "/supplier-by-id",
            MIX_SUPPLIER_BY_ID,
            &["id"],
            &[
                "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers WHERE id = :id",
            ],
        ),
        query_doc(
            "product-with-supplier",
            "Product with supplier",
            "/product-with-supplier",
            MIX_PRODUCT_WITH_SUPPLIER,
            &["id"],
            &[
                "SELECT p.*, s.* FROM products p INNER JOIN suppliers s ON p.supplier_id = s.id WHERE p.id = :id",
            ],
        ),
        query_doc(
            "order-with-details",
            "Order with details",
            "/order-with-details",
            MIX_ORDER_WITH_DETAILS,
            &["id"],
            &[
                "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = :id",
                "SELECT unit_price, quantity, discount, order_id, product_id FROM order_details WHERE order_id = :id",
            ],
        ),
        query_doc(
            "order-with-details-and-products",
            "Order with details and products",
            "/order-with-details-and-products",
            MIX_ORDER_WITH_DETAILS_AND_PRODUCTS,
            &["id"],
            &[
                "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = :id",
                "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name FROM order_details d LEFT JOIN products p ON d.product_id = p.id WHERE d.order_id = :id",
            ],
        ),
        query_doc(
            "customers",
            "Customers page",
            "/customers",
            MIX_CUSTOMERS,
            &["limit", "offset"],
            &[
                "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers ORDER BY id LIMIT :limit OFFSET :offset",
            ],
        ),
        query_doc(
            "employees",
            "Employees page",
            "/employees",
            MIX_EMPLOYEES,
            &["limit", "offset"],
            &[
                "SELECT id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id FROM employees ORDER BY id LIMIT :limit OFFSET :offset",
            ],
        ),
        query_doc(
            "suppliers",
            "Suppliers page",
            "/suppliers",
            MIX_SUPPLIERS,
            &["limit", "offset"],
            &[
                "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers ORDER BY id LIMIT :limit OFFSET :offset",
            ],
        ),
        query_doc(
            "products",
            "Products page",
            "/products",
            MIX_PRODUCTS,
            &["limit", "offset"],
            &[
                "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products ORDER BY id LIMIT :limit OFFSET :offset",
            ],
        ),
        query_doc(
            "orders-with-details",
            "Orders with detail totals",
            "/orders-with-details",
            MIX_ORDERS_WITH_DETAILS,
            &["limit", "offset"],
            &[
                "SELECT o.id, o.shipped_date, o.ship_name, o.ship_city, o.ship_country, count(d.product_id), COALESCE(sum(d.quantity), 0), COALESCE(sum(d.quantity * d.unit_price), 0) FROM orders o LEFT JOIN order_details d ON o.id = d.order_id GROUP BY o.id ORDER BY o.id LIMIT :limit OFFSET :offset",
            ],
        ),
        query_doc(
            "search-customer",
            "Search customers",
            "/search-customer",
            MIX_SEARCH_CUSTOMER,
            &["term"],
            &[
                "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE company_name LIKE :term",
            ],
        ),
        query_doc(
            "search-product",
            "Search products",
            "/search-product",
            MIX_SEARCH_PRODUCT,
            &["term"],
            &[
                "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products WHERE name LIKE :term",
            ],
        ),
    ]
}

fn query_catalog_for_requests(requests: &[RequestDoc]) -> Vec<QueryDoc> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for request in requests {
        *counts.entry(request.path.as_str()).or_default() += 1;
    }

    let mut known_paths = BTreeSet::new();
    let mut docs = query_catalog()
        .into_iter()
        .map(|mut query| {
            known_paths.insert(query.path.clone());
            query.mix = counts.get(query.path.as_str()).copied().unwrap_or(0);
            query
        })
        .filter(|query| query.mix > 0)
        .collect::<Vec<_>>();

    for (path, mix) in counts {
        if known_paths.contains(path) {
            continue;
        }
        docs.push(query_doc(
            &query_id_from_path(path),
            path.trim_start_matches('/'),
            path,
            mix,
            &[],
            &["Custom request path; SQL shape is not defined in the benchmark catalog."],
        ));
    }

    docs
}

fn query_id_from_path(path: &str) -> String {
    let id = path
        .trim_start_matches('/')
        .chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if id.is_empty() {
        "custom".to_string()
    } else {
        id
    }
}

fn query_doc(
    id: &str,
    name: &str,
    path: &str,
    mix: usize,
    params: &[&str],
    sql: &[&str],
) -> QueryDoc {
    QueryDoc {
        id: id.to_string(),
        name: name.to_string(),
        method: "GET".to_string(),
        path: path.to_string(),
        mix,
        params: params.iter().map(|param| (*param).to_string()).collect(),
        sql: sql
            .iter()
            .map(|text| QueryShapeDoc {
                dialect: "sql".to_string(),
                text: (*text).to_string(),
            })
            .collect(),
    }
}

fn write_manifest(
    run_dir: &Path,
    run_id: &str,
    input: &RunInput,
    ctx: ManifestContext<'_>,
) -> Result<(), Fail> {
    let sums = artifact_sums(run_dir)?;
    let mut sys = System::new_all();
    sys.refresh_memory();

    let total_duration_s: u32 = ctx.workload.stages.iter().map(|s| s.sec).sum();
    let max_vus = ctx
        .workload
        .stages
        .iter()
        .filter_map(|s| s.vus)
        .max()
        .unwrap_or(ctx.workload.load.concurrency);

    let manifest = ManifestDoc {
        version: "v1",
        run_id: run_id.to_string(),
        cohort_id: ctx.cohort_id.to_string(),
        name: run_name(input.suite, ctx.class),
        suite: input.suite.to_string(),
        git: input.git.clone(),
        workload: input.workload_hash.clone(),
        targets: input
            .targets
            .iter()
            .map(|target| target.id.clone())
            .collect(),
        target_meta: input.targets.iter().map(target_meta_doc).collect(),
        queries: query_catalog_for_requests(ctx.requests),
        start: ctx.start.to_string(),
        end: ctx.end.to_string(),
        status: Status::Success,
        seed: ctx.seed,
        load: LoadSummary {
            executor: ctx.workload.load.executor.clone(),
            stages: ctx.workload.stages.len() as u32,
            duration_s: total_duration_s,
            max_vus,
            requests: ctx.requests_count,
        },
        dataset: DatasetSummary {
            customers: crate::load::SEED_CUSTOMERS,
            employees: crate::load::SEED_EMPLOYEES,
            orders: crate::load::SEED_ORDERS,
            suppliers: crate::load::SEED_SUPPLIERS,
            products: crate::load::SEED_PRODUCTS,
            details_per_order: 6,
        },
        artifacts: Artifacts {
            base: ".".to_string(),
            summary: "targets/".to_string(),
            report: "reports/compare.md".to_string(),
            sums,
        },
        runner: Runner {
            class: class_name(ctx.class).to_string(),
            os: std::env::consts::OS.to_string(),
            cpu: std::env::consts::ARCH.to_string(),
            cores: std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(1),
            mem_gb: memory_gb(&sys),
            metrics: RunnerMetrics {
                cpu_scope: "host",
                memory_scope: "target_process_tree",
                network_scope: "unmeasured",
            },
            headroom: Headroom {
                cpu_peak: ctx.headroom.cpu_peak,
                net_peak: ctx.headroom.net_peak,
            },
        },
        trials: TrialMeta {
            count: input.trials,
            aggregate: "median",
        },
    };
    write_json(
        run_dir.join("manifest.json"),
        &manifest,
        Code::AggregateFail,
    )?;
    crate::schema::validate_manifest(&run_dir.join("manifest.json"))
}

fn artifact_sums(run_dir: &Path) -> Result<BTreeMap<String, String>, Fail> {
    let mut files = Vec::new();
    collect_files(run_dir, &mut files)?;
    let mut sums = BTreeMap::new();
    for path in files {
        if path.file_name() == Some(OsStr::new("manifest.json"))
            || path.file_name() == Some(OsStr::new("events.jsonl"))
            || path.file_name() == Some(OsStr::new("result.json"))
        {
            continue;
        }
        let rel = path
            .strip_prefix(run_dir)
            .map_err(|e| Fail::new(Code::AggregateFail, format!("strip prefix failed: {e}")))?
            .to_string_lossy()
            .replace('\\', "/");
        let sum = sha256_file(&path)?;
        sums.insert(rel, sum);
    }
    Ok(sums)
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Fail> {
    for entry in fs::read_dir(dir).map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("read_dir {} failed: {e}", dir.display()),
        )
    })? {
        let entry =
            entry.map_err(|e| Fail::new(Code::AggregateFail, format!("dir entry error: {e}")))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn compute_primary(measurements: &[TrialMeasurement]) -> PrimaryDoc {
    let aggregate_points: Vec<&Point> = measurements
        .iter()
        .map(|measurement| &measurement.aggregate)
        .collect();
    let sample_points: Vec<&Point> = measurements
        .iter()
        .flat_map(|measurement| measurement.series.iter())
        .collect();

    let rps_avg: Vec<f64> = aggregate_points.iter().map(|point| point.rps).collect();
    let rps_peak: Vec<f64> = sample_points.iter().map(|point| point.rps).collect();
    let err: Vec<f64> = aggregate_points.iter().map(|point| point.err).collect();
    let lat_avg: Vec<f64> = aggregate_points
        .iter()
        .map(|point| point.latency.avg)
        .collect();
    // p90 is interpolated from avg and p95 since Point only carries avg/p95/p99/p999.
    // Linear interpolation: p90 ≈ avg + (p95 - avg) * (0.90 / 0.95)
    let lat_p90: Vec<f64> = aggregate_points
        .iter()
        .map(|point| point.latency.avg + (point.latency.p95 - point.latency.avg) * (90.0 / 95.0))
        .collect();
    let lat_p95: Vec<f64> = aggregate_points
        .iter()
        .map(|point| point.latency.p95)
        .collect();
    let lat_p99: Vec<f64> = aggregate_points
        .iter()
        .map(|point| point.latency.p99)
        .collect();
    let lat_p999: Vec<f64> = aggregate_points
        .iter()
        .map(|point| point.latency.p999.unwrap_or(point.latency.p99))
        .collect();
    let cpu_avg: Vec<f64> = aggregate_points
        .iter()
        .map(|point| avg(&point.cpu))
        .collect();
    let cpu_peak: Vec<f64> = sample_points
        .iter()
        .flat_map(|point| point.cpu.iter().copied())
        .collect();
    let mem_avg: Vec<f64> = aggregate_points
        .iter()
        .filter_map(|point| point.mem_mb)
        .collect();
    let mem_peak: Vec<f64> = sample_points
        .iter()
        .filter_map(|point| point.mem_mb)
        .collect();

    PrimaryDoc {
        rps: AvgPeakDoc {
            avg: median(&rps_avg),
            peak: peak(&rps_peak),
        },
        latency: LatencyDoc {
            avg: median(&lat_avg),
            p90: median(&lat_p90),
            p95: median(&lat_p95),
            p99: median(&lat_p99),
            p999: median(&lat_p999),
        },
        cpu: AvgPeakDoc {
            avg: median(&cpu_avg),
            peak: peak(&cpu_peak),
        },
        mem: if mem_avg.is_empty() && mem_peak.is_empty() {
            None
        } else {
            Some(AvgPeakDoc {
                avg: median(&mem_avg),
                peak: peak(&mem_peak),
            })
        },
        err: median(&err),
    }
}

fn compute_spread(measurements: &[TrialMeasurement], trials: u32) -> SpreadDoc {
    let rps: Vec<f64> = measurements
        .iter()
        .map(|measurement| measurement.aggregate.rps)
        .collect();
    let p95: Vec<f64> = measurements
        .iter()
        .map(|measurement| measurement.aggregate.latency.p95)
        .collect();
    let cpu: Vec<f64> = measurements
        .iter()
        .map(|measurement| avg(&measurement.aggregate.cpu))
        .collect();
    let mem: Vec<f64> = measurements
        .iter()
        .filter_map(|measurement| measurement.aggregate.mem_mb)
        .collect();
    let err: Vec<f64> = measurements
        .iter()
        .map(|measurement| measurement.aggregate.err)
        .collect();
    SpreadDoc {
        trials,
        aggregate: "median",
        rps: RangeDoc {
            min: min(&rps),
            max: max(&rps),
        },
        p95: RangeDoc {
            min: min(&p95),
            max: max(&p95),
        },
        variance: VarianceDoc {
            rps: variance_metric(&rps),
            p95: variance_metric(&p95),
            cpu: variance_metric(&cpu),
            mem: if mem.is_empty() {
                None
            } else {
                Some(variance_metric(&mem))
            },
            err: variance_metric(&err),
        },
        boxplot: BoxPlotDoc {
            rps: box_metric(&rps),
            p95: box_metric(&p95),
            cpu: box_metric(&cpu),
            mem: if mem.is_empty() {
                None
            } else {
                Some(box_metric(&mem))
            },
            err: box_metric(&err),
        },
        ci95: ci95(&rps, &p95),
    }
}

fn compute_saturation(points: &[Point]) -> SaturationDoc {
    let mut knee = points
        .iter()
        .max_by(|a, b| a.rps.total_cmp(&b.rps))
        .map(|point| (point.rps, point.latency.p95))
        .unwrap_or((0.0, 0.0));

    for window in points.windows(2) {
        let prev = &window[0];
        let next = &window[1];
        let rps_delta = next.rps - prev.rps;
        if rps_delta <= 0.0 {
            continue;
        }
        let p95_delta = next.latency.p95 - prev.latency.p95;
        let slope = p95_delta / rps_delta;
        if slope > 0.02 {
            knee = (next.rps, next.latency.p95);
            break;
        }
    }

    SaturationDoc {
        knee_rps: knee.0,
        knee_p95: knee.1,
    }
}

fn compute_headroom(measurements: &BTreeMap<String, Vec<TrialMeasurement>>) -> Headroom {
    let mut cpu_peak: f64 = 0.0;
    for target_measurements in measurements.values() {
        for measurement in target_measurements {
            for point in &measurement.series {
                cpu_peak = cpu_peak.max(peak(&point.cpu));
            }
        }
    }
    Headroom {
        cpu_peak,
        net_peak: None,
    }
}

fn regression_gate(current: &BTreeMap<String, PrimaryDoc>, baseline: Option<&Baseline>) -> Gate {
    let Some(baseline) = baseline else {
        return Gate::Skip;
    };

    let mut compared = 0usize;
    for (target_id, head) in current {
        let Some(base) = baseline.summaries.get(target_id) else {
            continue;
        };
        compared += 1;

        let rps_drop_abs = base.rps.avg - head.rps.avg;
        let rps_drop_rel = if base.rps.avg > 0.0 {
            rps_drop_abs / base.rps.avg
        } else {
            0.0
        };

        let p95_rise_abs = head.latency.p95 - base.latency.p95;
        let p95_rise_rel = if base.latency.p95 > 0.0 {
            p95_rise_abs / base.latency.p95
        } else {
            0.0
        };

        let rps_fail = rps_drop_abs > 50.0 && rps_drop_rel > 0.10;
        let p95_fail = p95_rise_abs > 5.0 && p95_rise_rel > 0.10;
        if rps_fail || p95_fail {
            return Gate::Fail;
        }
    }

    if compared == 0 {
        Gate::Skip
    } else {
        Gate::Pass
    }
}

fn limits_gate(summaries: &BTreeMap<String, PrimaryDoc>, limits: &Limits) -> Gate {
    for summary in summaries.values() {
        if summary.err > limits.err {
            return Gate::Fail;
        }
        if let Some(p95_limit) = limits.p95
            && summary.latency.p95 > p95_limit
        {
            return Gate::Fail;
        }
    }
    Gate::Pass
}

fn gate_name(gate: &Gate) -> &'static str {
    match gate {
        Gate::Pass => "pass",
        Gate::Fail => "fail",
        Gate::Skip => "skip",
    }
}

fn validate_targets(targets: &[Target]) -> Result<(), Fail> {
    let mut ids = BTreeSet::new();
    for target in targets {
        if target.version != "v1" {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} has invalid version {}",
                    target.id, target.version
                ),
            ));
        }
        if target.id.is_empty() {
            return Err(Fail::new(Code::InvalidInput, "target id must not be empty"));
        }
        if !is_slug(&target.id) {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target id must match [a-z0-9][a-z0-9-]*: {}", target.id),
            ));
        }
        if target.display.name.trim().is_empty() {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} display.name must not be empty", target.id),
            ));
        }
        if target
            .display
            .description
            .as_ref()
            .is_some_and(|description| description.trim().is_empty())
        {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} display.description must not be empty when set",
                    target.id
                ),
            ));
        }
        if target.proc.workers == 0 {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} proc.workers must be >= 1", target.id),
            ));
        }
        if !matches!(target.proc.mode.as_str(), "single" | "multi") {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} proc.mode invalid: {}",
                    target.id, target.proc.mode
                ),
            ));
        }
        if target.pool.max == 0 {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} pool.max must be >= 1", target.id),
            ));
        }
        if let Some(min) = target.pool.min
            && min > target.pool.max
        {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} pool.min must be <= pool.max", target.id),
            ));
        }
        if let Some(acquire_ms) = target.pool.acquire_ms
            && acquire_ms == 0
        {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} pool.acquire_ms must be >= 1", target.id),
            ));
        }
        if target.fair.workers == 0 || target.fair.pool == 0 {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} fair.workers/fair.pool must be >= 1", target.id),
            ));
        }
        if target.fair.workers != target.proc.workers {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} fair.workers ({}) must match proc.workers ({})",
                    target.id, target.fair.workers, target.proc.workers
                ),
            ));
        }
        if target.fair.pool != target.pool.max {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} fair.pool ({}) must match pool.max ({})",
                    target.id, target.fair.pool, target.pool.max
                ),
            ));
        }
        if target.fair.db.is_empty() || target.fair.contract.is_empty() {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} fair.db/fair.contract must not be empty",
                    target.id
                ),
            ));
        }
        if !target.fair.schema.starts_with("sha256:") {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} fair.schema must start with sha256:", target.id),
            ));
        }
        if !target.db.hash.starts_with("sha256:") {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} db.hash must start with sha256:", target.id),
            ));
        }
        if target.db.profile.is_empty() {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} db.profile must not be empty", target.id),
            ));
        }
        if !matches!(
            target.wire.format.as_str(),
            "json" | "text" | "binary" | "bsatn"
        ) {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} wire.format invalid: {}",
                    target.id, target.wire.format
                ),
            ));
        }
        if !matches!(target.lang.as_str(), "ts" | "go" | "rust" | "other") {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} lang invalid: {}", target.id, target.lang),
            ));
        }
        if target.runtime.name.is_empty()
            || target.runtime.ver.is_empty()
            || target.orm.name.is_empty()
            || target.orm.ver.is_empty()
            || target.driver.name.is_empty()
            || target.driver.ver.is_empty()
        {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} runtime/orm/driver name and version must not be empty",
                    target.id
                ),
            ));
        }
        if target
            .driver
            .transport
            .as_ref()
            .is_some_and(String::is_empty)
        {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} driver.transport must not be empty", target.id),
            ));
        }
        if target.contract.ver.is_empty() {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("target {} contract.ver must not be empty", target.id),
            ));
        }
        if target.fair.contract != target.contract.ver {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "target {} fair.contract ({}) must match contract.ver ({})",
                    target.id, target.fair.contract, target.contract.ver
                ),
            ));
        }
        validate_exec(&target.id, "parity", &target.parity)?;
        validate_exec(&target.id, "load", &target.load)?;
        if let Some(spec) = &target.warmup {
            validate_exec(&target.id, "warmup", spec)?;
        }
        if let Some(spec) = &target.server {
            validate_exec(&target.id, "server", spec)?;
        }
        if !ids.insert(target.id.clone()) {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("duplicate target id: {}", target.id),
            ));
        }
    }
    Ok(())
}

fn validate_exec(target_id: &str, step: &str, spec: &Exec) -> Result<(), Fail> {
    if spec.cmd.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("target {target_id} {step}.cmd must not be empty"),
        ));
    }
    if spec.cmd.iter().any(|arg| arg.trim().is_empty()) {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("target {target_id} {step}.cmd entries must not be empty"),
        ));
    }
    if spec.cwd.as_ref().is_some_and(|cwd| cwd.trim().is_empty()) {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("target {target_id} {step}.cwd must not be empty when set"),
        ));
    }
    if spec.env.keys().any(|key| key.trim().is_empty()) {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("target {target_id} {step}.env keys must not be empty"),
        ));
    }
    if spec.timeout_s == Some(0) {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("target {target_id} {step}.timeout_s must be >= 1 when set"),
        ));
    }
    Ok(())
}

fn validate_workload(workload: &Workload) -> Result<(), Fail> {
    if workload.version != "v1" {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("workload version must be v1, got {}", workload.version),
        ));
    }
    if workload.stages.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            "workload.stages must contain at least one stage",
        ));
    }
    if !matches!(workload.load.kind.as_str(), "open" | "closed") {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("workload.load.kind invalid: {}", workload.load.kind),
        ));
    }
    if !matches!(
        workload.load.executor.as_str(),
        "constant-arrival-rate" | "ramping-arrival-rate" | "ramping-vus" | "constant-vus"
    ) {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("workload.load.executor invalid: {}", workload.load.executor),
        ));
    }
    if !is_duration_unit(&workload.load.unit) {
        return Err(Fail::new(
            Code::InvalidInput,
            format!("workload.load.unit invalid: {}", workload.load.unit),
        ));
    }
    if workload.data.name.is_empty() || workload.data.schema.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            "workload.data.name and workload.data.schema must not be empty",
        ));
    }
    if workload.requests.file.is_empty() {
        return Err(Fail::new(
            Code::InvalidInput,
            "workload.requests.file must not be empty",
        ));
    }
    if workload.sampling.cpu_ms < 50 || workload.sampling.bucket_s == 0 {
        return Err(Fail::new(
            Code::InvalidInput,
            "workload.sampling.cpu_ms must be >= 50 and bucket_s >= 1",
        ));
    }

    match workload.shape.mode.as_str() {
        "single" => {
            let valid = workload
                .shape
                .endpoint
                .as_ref()
                .is_some_and(|v| !v.is_empty());
            if !valid {
                return Err(Fail::new(
                    Code::InvalidInput,
                    "workload.shape.endpoint is required when mode=single",
                ));
            }
        }
        "mixed" => {
            if workload.shape.endpoint.is_some() {
                return Err(Fail::new(
                    Code::InvalidInput,
                    "workload.shape.endpoint must be null when mode=mixed",
                ));
            }
        }
        _ => {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("workload.shape.mode invalid: {}", workload.shape.mode),
            ));
        }
    }

    match workload.requests.source.as_str() {
        "generated" | "file" => {}
        _ => {
            return Err(Fail::new(
                Code::InvalidInput,
                format!(
                    "workload.requests.source invalid: {}",
                    workload.requests.source
                ),
            ));
        }
    }

    let skip = workload.requests.skip.iter().collect::<BTreeSet<_>>();
    if skip.len() != workload.requests.skip.len() {
        return Err(Fail::new(
            Code::InvalidInput,
            "workload.requests.skip must contain unique values",
        ));
    }
    if workload.requests.skip.iter().any(String::is_empty) {
        return Err(Fail::new(
            Code::InvalidInput,
            "workload.requests.skip entries must not be empty",
        ));
    }

    for (idx, stage) in workload.stages.iter().enumerate() {
        let has_vus = stage.vus.is_some();
        let has_rps = stage.rps.is_some();
        if has_vus == has_rps {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("workload.stages[{idx}] must set exactly one of vus or rps"),
            ));
        }
        if stage.sec == 0 {
            return Err(Fail::new(
                Code::InvalidInput,
                format!("workload.stages[{idx}].sec must be >= 1"),
            ));
        }
    }

    if !(0.0..=1.0).contains(&workload.limits.err) {
        return Err(Fail::new(
            Code::InvalidInput,
            "workload.limits.err must be between 0 and 1",
        ));
    }
    if workload.limits.p95.is_some_and(|p95| p95 <= 0.0) {
        return Err(Fail::new(
            Code::InvalidInput,
            "workload.limits.p95 must be > 0 when provided",
        ));
    }
    Ok(())
}

fn resolve_seed(class: Class, seed: Option<u64>, workload_seed: u64) -> u64 {
    match class {
        Class::Full | Class::Publish => workload_seed,
        Class::Small => seed.unwrap_or(workload_seed),
    }
}

fn is_slug(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(head) = chars.next() else {
        return false;
    };
    if !(head.is_ascii_lowercase() || head.is_ascii_digit()) {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn is_cohort_id(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(head) = chars.next() else {
        return false;
    };
    if !(head.is_ascii_lowercase() || head.is_ascii_digit()) {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
}

fn is_duration_unit(value: &str) -> bool {
    if value.len() < 2 {
        return false;
    }
    let (num, unit) = if let Some(num) = value.strip_suffix("ms") {
        (num, "ms")
    } else if let Some(num) = value.strip_suffix('s') {
        (num, "s")
    } else if let Some(num) = value.strip_suffix('m') {
        (num, "m")
    } else {
        return false;
    };
    !num.is_empty() && num.chars().all(|ch| ch.is_ascii_digit()) && !unit.is_empty()
}

fn must_exist(path: &Path) -> Result<(), Fail> {
    if path.exists() {
        Ok(())
    } else {
        Err(Fail::new(
            Code::InvalidInput,
            format!("missing input: {}", path.display()),
        ))
    }
}

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Fail> {
    load_json_with_code(path, Code::InvalidInput)
}

fn load_json_with_code<T: serde::de::DeserializeOwned>(path: &Path, code: Code) -> Result<T, Fail> {
    let body = fs::read_to_string(path)
        .map_err(|e| Fail::new(code, format!("failed to read {}: {e}", path.display())))?;
    serde_json::from_str::<T>(&body)
        .map_err(|e| Fail::new(code, format!("invalid json {}: {e}", path.display())))
}

fn memory_gb(sys: &System) -> f64 {
    (sys.total_memory() as f64) / (1024.0 * 1024.0 * 1024.0)
}

fn write_json(path: PathBuf, value: &impl serde::Serialize, code: Code) -> Result<(), Fail> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| Fail::new(code, format!("serialize {} failed: {e}", path.display())))?;
    fs::write(&path, body)
        .map_err(|e| Fail::new(code, format!("write {} failed: {e}", path.display())))
}

fn new_events_writer(path: &Path) -> Result<BufWriter<File>, Fail> {
    let file = File::create(path).map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("failed to create events {}: {e}", path.display()),
        )
    })?;
    Ok(BufWriter::new(file))
}

fn emit(
    writer: &mut BufWriter<File>,
    json: bool,
    level: &'static str,
    step: &'static str,
    msg: impl Into<String>,
) -> Result<(), Fail> {
    let event = Event {
        time: now_tag(),
        level,
        step,
        msg: msg.into(),
    };
    let line = serde_json::to_string(&event)
        .map_err(|e| Fail::new(Code::AggregateFail, format!("event encode failed: {e}")))?;
    writer
        .write_all(line.as_bytes())
        .and_then(|_| writer.write_all(b"\n"))
        .map_err(|e| Fail::new(Code::RunFail, format!("event write failed: {e}")))?;
    if json {
        println!("{line}");
    }
    Ok(())
}

fn now_tag() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn git_sha_short() -> String {
    let out = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output();
    let Ok(out) = out else {
        return "0000000".to_string();
    };
    if !out.status.success() {
        return "0000000".to_string();
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.len() >= 7 && text.chars().all(|ch| ch.is_ascii_hexdigit()) {
        text
    } else {
        "0000000".to_string()
    }
}

fn make_run_id(suite: &str, git: &str) -> String {
    let fmt = format_description::parse("[year][month][day]T[hour][minute][second]Z")
        .expect("valid timestamp format");
    let ts = OffsetDateTime::now_utc()
        .format(&fmt)
        .unwrap_or_else(|_| "19700101T000000Z".to_string());
    format!("{ts}_{git}_{suite}")
}

fn resolve_cohort_id(explicit: Option<&str>, run_id: &str) -> Result<String, Fail> {
    let cohort_id = explicit.unwrap_or(run_id).trim().to_ascii_lowercase();
    if is_cohort_id(&cohort_id) {
        Ok(cohort_id)
    } else {
        Err(Fail::new(
            Code::InvalidCli,
            format!(
                "cohort id must match [a-z0-9][a-z0-9_-]*, got {}",
                explicit.unwrap_or(run_id)
            ),
        ))
    }
}

fn sha256_file(path: &Path) -> Result<String, Fail> {
    let data = fs::read(path).map_err(|e| {
        Fail::new(
            Code::AggregateFail,
            format!("failed to read {} for sha256: {e}", path.display()),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(data);
    let sum = hasher.finalize();
    Ok(format!("sha256:{sum:x}"))
}

fn class_name(class: Class) -> &'static str {
    match class {
        Class::Small => "small",
        Class::Full => "full",
        Class::Publish => "publish",
    }
}

fn avg(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut items = values.to_vec();
    items.sort_by(f64::total_cmp);
    median_sorted(&items)
}

fn median_sorted(items: &[f64]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    let mid = items.len() / 2;
    if items.len() % 2 == 1 {
        items[mid]
    } else {
        (items[mid - 1] + items[mid]) / 2.0
    }
}

fn box_metric(values: &[f64]) -> BoxMetricDoc {
    if values.is_empty() {
        return BoxMetricDoc {
            min: 0.0,
            q1: 0.0,
            median: 0.0,
            q3: 0.0,
            max: 0.0,
            samples: 0,
        };
    }

    let mut items = values.to_vec();
    items.sort_by(f64::total_cmp);
    let mid = items.len() / 2;
    let (lower, upper) = if items.len().is_multiple_of(2) {
        (&items[..mid], &items[mid..])
    } else {
        (&items[..mid], &items[mid + 1..])
    };
    let median = median_sorted(&items);

    BoxMetricDoc {
        min: items[0],
        q1: if lower.is_empty() {
            median
        } else {
            median_sorted(lower)
        },
        median,
        q3: if upper.is_empty() {
            median
        } else {
            median_sorted(upper)
        },
        max: *items.last().unwrap_or(&0.0),
        samples: items.len() as u32,
    }
}

fn ci95(rps: &[f64], p95: &[f64]) -> Option<CiDoc> {
    let rps = bootstrap(rps)?;
    let p95 = bootstrap(p95)?;
    Some(CiDoc { rps, p95 })
}

fn bootstrap(values: &[f64]) -> Option<RangeDoc> {
    if values.len() < 2 {
        return None;
    }

    let mut seed = 0_u64;
    for value in values {
        seed ^= value.to_bits().rotate_left(13);
        seed = seed.rotate_left(7).wrapping_add(0x9e37_79b9_7f4a_7c15);
    }

    let mut rng = StdRng::seed_from_u64(seed);
    let mut out = Vec::with_capacity(512);
    for _ in 0..512 {
        let mut sample = Vec::with_capacity(values.len());
        for _ in 0..values.len() {
            let idx = rng.random_range(0..values.len());
            sample.push(values[idx]);
        }
        out.push(median(&sample));
    }
    out.sort_by(f64::total_cmp);

    let lo = ((out.len() as f64 - 1.0) * 0.025).round() as usize;
    let hi = ((out.len() as f64 - 1.0) * 0.975).round() as usize;
    Some(RangeDoc {
        min: out[lo],
        max: out[hi],
    })
}

fn variance_metric(values: &[f64]) -> VarianceMetricDoc {
    let value = sample_variance(values);
    VarianceMetricDoc {
        value,
        stdev: value.sqrt(),
        samples: values.len() as u32,
    }
}

fn sample_variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = avg(values);
    values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() - 1) as f64
}

fn peak(values: &[f64]) -> f64 {
    values.iter().fold(0.0_f64, |acc, val| acc.max(*val))
}

fn min(values: &[f64]) -> f64 {
    values.iter().copied().reduce(f64::min).unwrap_or(0.0)
}

fn max(values: &[f64]) -> f64 {
    values.iter().copied().reduce(f64::max).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::{
        MIX_SEARCH_CUSTOMER, MIX_SEARCH_PRODUCT, TrialMeasurement, box_metric, combined_series,
        compute_primary, compute_spread, query_catalog_total_mix, request_path_skipped,
        resolve_seed, sample_variance,
    };
    use crate::cli::Class;
    use crate::model::{Latency, Point};

    #[test]
    fn publish_seed_ignores_cli_override() {
        assert_eq!(resolve_seed(Class::Publish, Some(42), 17), 17);
        assert_eq!(resolve_seed(Class::Publish, None, 17), 17);
    }

    #[test]
    fn full_seed_ignores_cli_override() {
        assert_eq!(resolve_seed(Class::Full, Some(42), 17), 17);
        assert_eq!(resolve_seed(Class::Full, None, 17), 17);
    }

    #[test]
    fn small_seed_prefers_cli_override() {
        assert_eq!(resolve_seed(Class::Small, Some(42), 17), 42);
        assert_eq!(resolve_seed(Class::Small, None, 17), 17);
    }

    #[test]
    fn summary_uses_sample_peak_and_full_series() {
        let measurements = vec![TrialMeasurement {
            aggregate: point(150.0, 10.0),
            series: vec![point(100.0, 20.0), point(250.0, 80.0)],
        }];

        let primary = compute_primary(&measurements);
        assert_eq!(primary.rps.avg, 150.0);
        assert_eq!(primary.rps.peak, 250.0);
        assert_eq!(primary.cpu.peak, 80.0);
        assert_eq!(combined_series(&measurements).len(), 2);
    }

    #[test]
    fn spread_reports_sample_variance() {
        let measurements = vec![
            TrialMeasurement {
                aggregate: point(100.0, 10.0),
                series: vec![point(100.0, 10.0)],
            },
            TrialMeasurement {
                aggregate: point(200.0, 20.0),
                series: vec![point(200.0, 20.0)],
            },
            TrialMeasurement {
                aggregate: point(300.0, 30.0),
                series: vec![point(300.0, 30.0)],
            },
        ];

        let spread = compute_spread(&measurements, 3);
        assert_eq!(
            spread.variance.rps.value,
            sample_variance(&[100.0, 200.0, 300.0])
        );
        assert_eq!(spread.variance.rps.stdev, 100.0);
        assert_eq!(spread.variance.rps.samples, 3);
        assert_eq!(spread.boxplot.rps.min, 100.0);
        assert_eq!(spread.boxplot.rps.q1, 100.0);
        assert_eq!(spread.boxplot.rps.median, 200.0);
        assert_eq!(spread.boxplot.rps.q3, 300.0);
        assert_eq!(spread.boxplot.rps.max, 300.0);
        assert_eq!(box_metric(&[100.0, 200.0, 300.0, 400.0]).q1, 150.0);
        assert_eq!(box_metric(&[100.0, 200.0, 300.0, 400.0]).q3, 350.0);
    }

    #[test]
    fn request_skip_matches_path_prefixes() {
        let skip = vec!["/search".to_string()];
        assert!(request_path_skipped("/search-customer", &skip));
        assert!(request_path_skipped("/search-product", &skip));
        assert!(!request_path_skipped("/customer-by-id", &skip));
    }

    #[test]
    fn generated_mix_matches_drizzle_benchmarks() {
        assert_eq!(query_catalog_total_mix(), 426_999);
        assert_eq!(
            query_catalog_total_mix() - MIX_SEARCH_CUSTOMER - MIX_SEARCH_PRODUCT,
            371_999
        );
    }

    fn point(rps: f64, cpu: f64) -> Point {
        Point {
            time: "2026-01-01T00:00:00Z".to_string(),
            rps,
            err: 0.0,
            latency: Latency {
                avg: 1.0,
                p95: 2.0,
                p99: 3.0,
                p999: Some(4.0),
            },
            cpu: vec![cpu],
            mem_mb: Some(64.0),
            queries: Vec::new(),
        }
    }
}
