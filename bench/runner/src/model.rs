use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workload {
    pub version: String,
    pub suite: String,
    pub load: Load,
    pub data: Data,
    pub shape: Shape,
    pub stages: Vec<Stage>,
    pub requests: Requests,
    pub sampling: Sampling,
    pub limits: Limits,
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
pub struct Sampling {
    pub cpu_ms: u32,
    pub bucket_s: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub err: f64,
    pub p95: Option<f64>,
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
#[allow(dead_code)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci95: Option<CiDoc>,
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
pub struct CiDoc {
    pub rps: RangeDoc,
    pub p95: RangeDoc,
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
    pub interval_s: u32,
    pub points: Vec<Point>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queries: Vec<QueryPoint>,
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
    pub p95: f64,
    pub p99: f64,
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
    pub cpu: String,
    pub cores: u32,
    pub mem_gb: f64,
    pub metrics: RunnerMetrics,
    pub headroom: Headroom,
}

#[derive(Debug, Serialize)]
pub struct RunnerMetrics {
    pub cpu_scope: &'static str,
    pub memory_scope: &'static str,
    pub network_scope: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Headroom {
    pub cpu_peak: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_peak: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TrialMeta {
    pub count: u32,
    pub aggregate: &'static str,
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
