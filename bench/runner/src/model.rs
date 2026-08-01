use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Default number of leading seconds tagged `phase=warmup` and excluded from
/// primary aggregates when a workload does not declare `warmup_s`.
pub const DEFAULT_WARMUP_S: u32 = 10;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workload {
    pub version: String,
    pub suite: String,
    pub name: String,
    pub load: Load,
    pub data: Data,
    pub shape: Shape,
    pub stages: Vec<Stage>,
    /// Leading seconds of the run marked `phase=warmup`. Excluded from primary
    /// aggregates but still emitted in the timeseries. Defaults to
    /// [`DEFAULT_WARMUP_S`].
    #[serde(default)]
    pub warmup_s: Option<u32>,
    pub requests: Requests,
    pub pacing: Pacing,
    pub sampling: Sampling,
    pub limits: Limits,
}

impl Workload {
    pub fn warmup_seconds(&self) -> u32 {
        self.warmup_s.unwrap_or(DEFAULT_WARMUP_S)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Load {
    pub kind: String,
    pub executor: String,
    pub unit: String,
    pub concurrency: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub name: String,
    pub seed: u64,
    pub schema: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub mode: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub sec: u32,
    pub vus: Option<u32>,
    pub rps: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requests {
    pub source: String,
    pub file: String,
    pub skip: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pacing {
    pub mode: PacingMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PacingMode {
    DrizzleBenchmark,
    None,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sampling {
    pub cpu_ms: u32,
    pub bucket_s: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub err: f64,
    pub p95: Option<f64>,
    /// Optional hard ceiling on mean-across-cores CPU (headroom gate). When
    /// absent the headroom gate is informational only: the closed-loop ramp
    /// saturates a colocated host by design, so "CPU touched 100%" is a
    /// property of the methodology, not a defect. Set this only on topologies
    /// where genuine headroom is expected (e.g. a dedicated two-host setup).
    #[serde(default)]
    pub cpu_mean_peak: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub version: String,
    pub id: String,
    pub display: DisplayMeta,
    pub lang: String,
    #[serde(default)]
    pub group: Option<String>,
    /// How the target reaches its data: `sql-roundtrip` (default) or
    /// `in-process-cache`. Targets that serve from a replicated in-process
    /// cache are exempt from the cross-target `fair` block equality check.
    #[serde(default)]
    pub data_access: Option<String>,
    /// Free-form note describing an intentional deviation from the canonical
    /// SQL shape for a route (for example "orm-idiomatic relation load").
    #[serde(default)]
    pub sql_variant: Option<String>,
    pub runtime: NameVer,
    pub orm: NameVer,
    pub driver: Driver,
    pub proc: Proc,
    pub pool: Pool,
    pub db: Db,
    pub wire: Wire,
    pub fair: Fair,
    pub contract: Contract,
    pub parity: Exec,
    #[serde(default)]
    pub warmup: Option<Exec>,
    pub load: Exec,
    #[serde(default)]
    pub server: Option<Exec>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DisplayMeta {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Exec {
    pub cmd: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub timeout_s: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NameVer {
    pub name: String,
    pub ver: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Driver {
    pub name: String,
    pub ver: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Proc {
    pub mode: String,
    pub workers: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Pool {
    pub max: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquire_ms: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Db {
    pub profile: String,
    pub hash: String,
    /// Whether the target uses server-side prepared statements.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub format: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Fair {
    pub workers: u32,
    pub pool: u32,
    pub db: String,
    pub schema: String,
    pub contract: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub ver: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RequestDoc {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct ResultDoc {
    pub version: &'static str,
    pub run_id: String,
    pub cohort_id: String,
    pub status: Status,
    pub suite: String,
    pub class: String,
    pub trials: u32,
    pub targets: usize,
    pub requests: usize,
    pub gates: Gates,
}

/// Contract vocabulary from the manifest/result schemas. The runner exits
/// non-zero rather than writing a failed artifact, so only `Success` is
/// constructed today; the remaining variants exist because consumers must be
/// able to parse them.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Status {
    Success,
    Failed,
    Canceled,
    Partial,
}

#[derive(Debug, Serialize)]
pub struct Gates {
    pub parity: Gate,
    pub headroom: Gate,
    pub regression: Gate,
    pub limits: Gate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub time: String,
    pub level: &'static str,
    pub step: &'static str,
    pub msg: String,
}

#[derive(Debug, Serialize)]
pub struct SummaryDoc {
    pub version: &'static str,
    pub run_id: String,
    pub suite: String,
    pub target_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub primary: PrimaryDoc,
    pub spread: SpreadDoc,
    pub saturation: SaturationDoc,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrimaryDoc {
    pub rps: AvgPeakDoc,
    pub latency: LatencyDoc,
    pub cpu: AvgPeakDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem: Option<AvgPeakDoc>,
    pub err: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AvgPeakDoc {
    pub avg: f64,
    pub peak: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LatencyDoc {
    pub avg: f64,
    /// Measured median. Optional only so that pre-`p50` baselines still parse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p50: Option<f64>,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SpreadDoc {
    pub trials: u32,
    pub aggregate: &'static str,
    pub rps: RangeDoc,
    pub p95: RangeDoc,
    pub variance: VarianceDoc,
    pub boxplot: BoxPlotDoc,
}

#[derive(Debug, Serialize, Clone)]
pub struct VarianceDoc {
    pub rps: VarianceMetricDoc,
    pub p95: VarianceMetricDoc,
    pub cpu: VarianceMetricDoc,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem: Option<VarianceMetricDoc>,
    pub err: VarianceMetricDoc,
}

#[derive(Debug, Serialize, Clone)]
pub struct VarianceMetricDoc {
    pub value: f64,
    pub stdev: f64,
    pub samples: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct BoxPlotDoc {
    pub rps: BoxMetricDoc,
    pub p95: BoxMetricDoc,
    pub cpu: BoxMetricDoc,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem: Option<BoxMetricDoc>,
    pub err: BoxMetricDoc,
}

#[derive(Debug, Serialize, Clone)]
pub struct BoxMetricDoc {
    pub min: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub max: f64,
    pub samples: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct RangeDoc {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SaturationDoc {
    pub knee_rps: f64,
    pub knee_p95: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeseriesDoc {
    pub version: &'static str,
    pub run_id: String,
    pub suite: String,
    pub target_id: String,
    /// Deprecated alias for [`Self::bucket_s`]. The name implied one continuous
    /// timeline, which stopped being true once trials were concatenated. Kept
    /// so existing consumers do not break.
    pub interval_s: u32,
    /// Nominal sampling bucket length. Points from every trial are concatenated
    /// into `points`, so consumers must segment on `Point::trial` rather than
    /// assume a single uninterrupted timeline.
    pub bucket_s: u32,
    pub points: Vec<Point>,
}

/// Which part of the load profile a bucket belongs to. Only `hold` buckets
/// feed the primary aggregates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// Leading `workload.warmup_s` seconds of the run.
    Warmup,
    /// A stage whose VU count is interpolating toward its target.
    Ramp,
    /// A stage sitting at its declared VU count.
    Hold,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Point {
    pub time: String,
    pub rps: f64,
    pub err: f64,
    pub latency: Latency,
    pub cpu: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem_mb: Option<f64>,
    /// Zero-based trial index this bucket came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trial: Option<u32>,
    /// Zero-based `workload.stages` index this bucket came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<Phase>,
    /// Completed requests counted in this bucket (the aggregation weight).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queries: Vec<QueryPoint>,
}

impl Point {
    /// Buckets without a `phase` tag come from an external load command that
    /// predates phase tagging; treat them as steady state.
    pub fn is_hold(&self) -> bool {
        self.phase.is_none_or(|phase| phase == Phase::Hold)
    }

    /// Aggregation weight: real request count when present, otherwise fall back
    /// to rps (equivalent under uniform bucket length).
    pub fn weight(&self) -> f64 {
        self.requests.map_or(self.rps, |count| count as f64)
    }

    /// Bucket wall seconds implied by `requests / rps`.
    pub fn wall_s(&self) -> f64 {
        match (self.requests, self.rps) {
            (Some(count), rps) if rps > 0.0 => count as f64 / rps,
            _ => 1.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryPoint {
    pub method: String,
    pub path: String,
    pub rps: f64,
    pub err: f64,
    pub latency: Latency,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Latency {
    pub avg: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p50: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p90: Option<f64>,
    pub p95: f64,
    pub p99: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p999: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ManifestDoc {
    pub version: &'static str,
    pub run_id: String,
    pub cohort_id: String,
    pub name: String,
    pub suite: String,
    pub git: String,
    pub workload: String,
    pub targets: Vec<String>,
    pub target_meta: Vec<TargetMetaDoc>,
    pub queries: Vec<QueryDoc>,
    pub start: String,
    pub end: String,
    pub status: Status,
    pub seed: u64,
    pub load: LoadSummary,
    pub dataset: DatasetSummary,
    pub artifacts: Artifacts,
    pub runner: Runner,
    pub trials: TrialMeta,
    /// Cross-target fairness findings for this run. Empty when every target
    /// declares the same `fair` block (or is exempt).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fairness: Vec<FairnessWarning>,
}

#[derive(Debug, Serialize)]
pub struct FairnessWarning {
    pub kind: String,
    pub msg: String,
}

#[derive(Debug, Serialize)]
pub struct TargetMetaDoc {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_variant: Option<String>,
    pub runtime: NameVer,
    pub orm: NameVer,
    pub driver: Driver,
    pub proc: Proc,
    pub pool: Pool,
    pub db: Db,
    pub wire: Wire,
    pub fair: Fair,
    pub contract: Contract,
}

#[derive(Debug, Serialize)]
pub struct QueryDoc {
    pub id: String,
    pub name: String,
    pub method: String,
    pub path: String,
    pub mix: usize,
    pub params: Vec<String>,
    pub sql: Vec<QueryShapeDoc>,
}

#[derive(Debug, Serialize)]
pub struct QueryShapeDoc {
    pub dialect: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct LoadSummary {
    pub executor: String,
    pub stages: u32,
    pub duration_s: u32,
    pub max_vus: u32,
    pub pacing: PacingMode,
    pub requests: usize,
}

#[derive(Debug, Serialize)]
pub struct DatasetSummary {
    pub customers: usize,
    pub employees: usize,
    pub orders: usize,
    pub suppliers: usize,
    pub products: usize,
    pub details_per_order: usize,
}

#[derive(Debug, Serialize)]
pub struct Artifacts {
    pub base: String,
    pub summary: String,
    pub report: String,
    pub sums: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct Runner {
    pub class: String,
    pub os: String,
    /// CPU brand string reported by the host (falls back to the target arch).
    pub cpu: String,
    /// Logical cores.
    pub cores: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores_physical: Option<u32>,
    pub mem_gb: f64,
    pub metrics: RunnerMetrics,
    pub headroom: Headroom,
    pub topology: Topology,
}

#[derive(Debug, Serialize)]
pub struct RunnerMetrics {
    pub cpu_scope: &'static str,
    pub memory_scope: &'static str,
    pub network_scope: &'static str,
}

/// Where the load generator, the target process, and the database sit relative
/// to each other. Everything the runner spawns shares one host today, so the
/// honest answer is recorded rather than implied.
#[derive(Debug, Serialize)]
pub struct Topology {
    /// Always true: the load generator runs on the same host as the target.
    pub loadgen_colocated: bool,
    /// True when the database also runs on this host (no remote DATABASE_URL).
    pub db_colocated: bool,
    /// Human-readable cpuset assignment, or null when nothing was pinned.
    pub cpu_pinning: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Headroom {
    /// Peak utilisation of any *single* core. Kept as `cpu_peak` because that
    /// is what the dashboard renders under that name; it is a diagnostic, not
    /// the gate input — one saturated core on a 16-core host says nothing about
    /// whether the host had headroom.
    pub cpu_peak: f64,
    /// Peak, over all samples, of CPU utilisation averaged across cores. This
    /// is what the publish headroom gate compares.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_mean_peak: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_peak: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TrialMeta {
    pub count: u32,
    pub aggregate: &'static str,
}

/// The subset of a written manifest that later steps read back: publishing to
/// the run index, and deciding whether a candidate run is a valid regression
/// baseline (`suite` + `workload` + `runner.class` must all match).
#[derive(Debug, Deserialize)]
pub struct ManifestSummary {
    pub run_id: String,
    pub cohort_id: String,
    pub name: String,
    pub suite: String,
    pub status: String,
    pub git: String,
    pub workload: String,
    pub start: String,
    pub end: String,
    pub targets: Vec<String>,
    pub runner: ManifestSummaryRunner,
}

#[derive(Debug, Deserialize)]
pub struct ManifestSummaryRunner {
    pub class: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunIndex {
    pub version: String,
    pub runs: Vec<RunIndexEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunIndexEntry {
    pub run_id: String,
    pub cohort_id: String,
    pub name: String,
    pub suite: String,
    pub status: String,
    pub class: String,
    pub git: String,
    pub start: String,
    pub end: String,
    pub targets: Vec<String>,
}
