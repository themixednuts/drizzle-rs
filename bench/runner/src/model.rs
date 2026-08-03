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
    /// Declares this workload as a capacity measurement. Present only on the
    /// stepped unpaced ramp; a paced workload cannot produce a capacity number
    /// (see [`Pacing`]) and the runner refuses the combination.
    #[serde(default)]
    pub saturation: Option<SaturationSpec>,
}

impl Workload {
    pub fn warmup_seconds(&self) -> u32 {
        self.warmup_s.unwrap_or(DEFAULT_WARMUP_S)
    }
}

/// A capacity measurement declared by the workload.
///
/// The steps are not listed here: they *are* the hold stages of
/// [`Workload::stages`], so the ramp has exactly one definition and the spec
/// cannot drift from what the load generator runs.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaturationSpec {
    pub slo: Slo,
}

/// The latency ceiling a step must stay under to be eligible as the peak.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Slo {
    pub metric: SloMetric,
    pub ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SloMetric {
    P50,
    P90,
    P95,
    P99,
}

impl SloMetric {
    pub fn of(self, latency: &StepLatencyDoc) -> f64 {
        match self {
            Self::P50 => latency.p50,
            Self::P90 => latency.p90,
            Self::P95 => latency.p95,
            Self::P99 => latency.p99,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::P50 => "p50",
            Self::P90 => "p90",
            Self::P95 => "p95",
            Self::P99 => "p99",
        }
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

#[derive(Debug, Clone, Copy, Deserialize)]
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

/// The harness knobs a target declares, and the family it competes in.
///
/// Within a family these must be identical or the run fails: a ranked
/// library-vs-library table is only meaningful when every entry ran the same
/// harness. Across families they are free to differ — that difference *is* the
/// stack comparison — and the manifest records it per family so a reader cannot
/// mistake a stack difference for a library one.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Fair {
    /// Database family this target competes in, e.g. `sqlite`, `turso`,
    /// `postgres`. Declared rather than inferred: `db.profile` distinguishes
    /// configurations *within* a family (prepared vs unprepared) and `fair.db`
    /// names the SQL dialect, so neither identifies the competition bracket.
    pub family: String,
    pub workers: u32,
    pub pool: u32,
    pub db: String,
    pub schema: String,
    pub contract: String,
    /// One-line summary of the engine tuning this target runs under (pragmas,
    /// server image, cache settings). Compared for equality within a family.
    pub tuning: String,
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
    /// Present only when the workload declared a capacity measurement. A paced
    /// run has no capacity number, and absent is the honest answer — consumers
    /// render it as "not measured", never as zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saturation: Option<SaturationDoc>,
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

/// Peak throughput under a declared latency SLO, plus the curve it was read off.
///
/// Replaces the pre-contract `{knee_rps, knee_p95}` heuristic, which inferred a
/// knee from a p95-doubling rule over a *paced* run and fell back to the
/// highest-throughput bucket when no knee appeared. That fallback reported a
/// capacity-shaped number for a run whose throughput was capped by its own sleep
/// timer. The three outcomes below exist so "we could not measure it" is always
/// said out loud instead.
#[derive(Debug, Serialize, Clone)]
pub struct SaturationDoc {
    pub slo: Slo,
    pub outcome: Outcome,
    /// The peak step. Present only when `outcome == "saturated"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak: Option<PeakDoc>,
    /// Throughput of the top step, which still met the SLO. Present only when
    /// `outcome == "did_not_saturate"`: capacity is at *least* this, and the
    /// ramp ended before it found the knee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower_bound_rps: Option<f64>,
    /// Every measured step, ascending by concurrency. The shape of
    /// rps-vs-concurrency is what makes the headline checkable.
    pub curve: Vec<CurveStepDoc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// A peak step was found and a later step failed to qualify.
    Saturated,
    /// No step qualified as a peak: every one either breached the SLO or was
    /// disqualified by its error rate. There is no peak, and none is reported.
    SloNeverMet,
    /// Every step up to and including the last one qualified. The top step is a
    /// lower bound, not a peak — the ramp was too short.
    DidNotSaturate,
}

#[derive(Debug, Serialize, Clone)]
pub struct PeakDoc {
    pub concurrency: u32,
    pub rps: f64,
    pub latency: StepLatencyDoc,
    pub cpu: f64,
    pub err: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct CurveStepDoc {
    pub concurrency: u32,
    pub rps: f64,
    pub latency: StepLatencyDoc,
    pub err: f64,
    pub cpu: f64,
    pub slo_met: bool,
    /// Why this step can never be the peak, or `null`. Always serialized so a
    /// consumer can rely on the key being present.
    pub disqualified: Option<String>,
}

impl CurveStepDoc {
    /// A step is eligible to be the peak only when it met the SLO *and* stayed
    /// within the error limit.
    pub fn qualifies(&self) -> bool {
        self.slo_met && self.disqualified.is_none()
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct StepLatencyDoc {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
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
    /// Virtual users in flight. With `pacing.mode = none` there is no think
    /// time, so this is also the request concurrency — the x-axis of the
    /// saturation curve. Absent on aggregates, which span several VU counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vus: Option<u32>,
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
    /// The harness each database family ran under, one entry per family present
    /// in this run. Within-family drift is a hard error, so every block here
    /// records a verified fact rather than an aspiration.
    pub harness: Vec<HarnessDoc>,
}

#[derive(Debug, Serialize)]
pub struct HarnessDoc {
    pub family: String,
    /// Targets whose declared harness was compared for equality.
    pub targets: Vec<String>,
    /// Absent only when every target in the family is exempt from the check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuning: Option<String>,
    /// True when at least one target was compared and they all agreed. False
    /// only when there was nothing to compare, in which case `workers`, `pool`
    /// and `tuning` are absent — never a claim that drift was tolerated.
    pub within_family_identical: bool,
    /// Targets excluded from the equality check because their configuration is
    /// not comparable by design (an in-process cache has no connection pool).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exempt: Vec<String>,
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
