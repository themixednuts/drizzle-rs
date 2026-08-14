export interface RunIndex {
	version: string;
	runs: RunIndexEntry[];
}

export interface RunIndexEntry {
	run_id: string;
	/** Absent on runs published before the runner grouped shards into cohorts. */
	cohort_id?: string;
	/** Absent on runs published before runs carried a display name. */
	name?: string;
	suite: string;
	status: string;
	class: string;
	git: string;
	start: string;
	end: string;
	targets: string[];
}

export interface RunCohort {
	id: string;
	name: string;
	suite: string;
	status: string;
	class: string;
	git: string;
	start: string;
	end: string;
	run_ids: string[];
	representative_run_id: string;
	targets: string[];
	result_count: number;
}

export interface Manifest {
	version: string;
	run_id: string;
	/** Absent on artifacts published before runs were grouped into cohorts. */
	cohort_id?: string;
	/** Absent on artifacts published before runs carried a display name. */
	name?: string;
	suite: string;
	git: string;
	workload: string;
	targets: string[];
	/** Absent on artifacts published before the runner emitted per-target declarations. */
	target_meta?: TargetMeta[];
	/** Absent on artifacts published before the runner recorded the query catalog. */
	queries?: QueryDoc[];
	/**
	 * The harness each database family ran under. Absent on artifacts published before the runner
	 * declared it — which the UI states as "not declared", never as an assumed default.
	 */
	harness?: HarnessFamily[];
	start: string;
	end: string;
	status: string;
	seed: number;
	load: {
		executor: string;
		stages: number;
		duration_s: number;
		max_vus: number;
		/** Absent on artifacts published before pacing was declared. */
		pacing?: 'drizzle-benchmark' | 'none';
		requests: number;
	};
	dataset: {
		customers: number;
		employees: number;
		orders: number;
		suppliers: number;
		products: number;
		details_per_order: number;
	};
	artifacts: {
		base: string;
		summary?: string;
		report?: string;
		sums: Record<string, string>;
	};
	runner: {
		class: string;
		os: string;
		cpu: string;
		cores: number;
		mem_gb: number;
		/** Absent on artifacts published before the runner declared what its metrics cover. */
		metrics?: { cpu_scope: string; memory_scope: string; network_scope: string };
		headroom: {
			/** Highest single-core utilization observed. Load, not spare capacity. */
			cpu_peak: number;
			/**
			 * Peak of the mean-across-cores utilization — the figure the runner's publish gate is
			 * written against. Absent on artifacts produced before the runner reported it.
			 */
			cpu_mean_peak?: number;
			net_peak?: number;
		};
		/**
		 * Where the load generator, the target and the database sat relative to each other.
		 *
		 * Absent on artifacts published before the runner recorded it. `cpu_pinning` is null whenever
		 * nothing was pinned — which is always the case on macOS and Windows, because Darwin exposes
		 * no usable CPU-affinity API and the runner's affinity call is Linux-only. The UI reports that
		 * absence rather than implying an isolation that did not happen.
		 */
		topology?: {
			loadgen_colocated: boolean;
			db_colocated: boolean;
			cpu_pinning: string | null;
		};
	};
	trials: { count: number; aggregate: string };
}

/**
 * The harness one database family ran under.
 *
 * Two distinct meanings of "fair" live here and must never be blurred. *Within* a family the
 * harness is enforced identical, which is what makes a row-to-row difference attributable to the
 * library. *Across* families the harness deliberately differs — that difference IS the stack
 * comparison — so it is recorded and displayed rather than constrained.
 */
export interface HarnessFamily {
	/** Database family, in the same vocabulary as `DbProfile`. */
	family: string;
	workers: number;
	pool: number;
	/** Free-text summary of the database-side tuning, e.g. "stock postgres:18-alpine". */
	tuning: string;
	/**
	 * Whether the runner verified every target in this family declared the same harness. A `false`
	 * is a real finding and is shown as a warning, never hidden.
	 */
	within_family_identical: boolean;
	/** The targets whose harness was compared. Optional in the schema; the runner emits it. */
	targets?: string[];
	/**
	 * Targets deliberately left out of the identity check.
	 *
	 * Load-bearing: `within_family_identical: true` alongside a non-empty `exempt` means "identical
	 * among the ones we checked", which is a weaker claim than it reads as. The UI names the
	 * exempted targets rather than letting the tick stand unqualified.
	 */
	exempt?: string[];
}

/** How a target answers a request: a real database round trip, or an in-process cache. */
export type DataAccess = 'sql-roundtrip' | 'in-process-cache';

export interface TargetMeta {
	id: string;
	name: string;
	description?: string;
	group?: string;
	lang: string;
	runtime: NameVer;
	orm: NameVer;
	driver: DriverMeta;
	proc: ProcMeta;
	pool: PoolMeta;
	db: DbMeta;
	wire: WireMeta;
	fair: FairMeta;
	contract: ContractMeta;
	/**
	 * Optional (runner spec v1 additive fields). Absent on artifacts produced before
	 * the fields were introduced — always treat `undefined` as "unknown", never as a value.
	 */
	data_access?: DataAccess;
	sql_variant?: string;
	/** Set by the dashboard when a manifest is missing target_meta for a summarized target. */
	incomplete?: boolean;
}

export interface NameVer {
	name: string;
	ver: string;
}

export interface DriverMeta extends NameVer {
	transport?: string;
}

export interface ProcMeta {
	mode: string;
	workers: number;
}

export interface PoolMeta {
	max: number;
	min?: number;
	acquire_ms?: number;
}

export interface DbMeta {
	profile: string;
	hash: string;
	/** Optional additive field: whether the target uses prepared statements. */
	prepared?: boolean;
}

export interface WireMeta {
	format: string;
}

export interface FairMeta {
	workers: number;
	pool: number;
	db: string;
	schema: string;
	contract: string;
	/**
	 * The comparison group this target claims membership of — the set of targets asserting they are
	 * directly comparable to each other.
	 *
	 * Usually the database engine, but NOT the same thing. It splits wherever the harness genuinely
	 * cannot be equalised: `bun-sqlite` is synchronous on a single-threaded runtime, so a pool of 8
	 * there would be theatre and forcing one would cripple the target rather than make it fair. So
	 * it sits in `sqlite-ts` with `drizzle-orm-sqlite` — same runtime, same pool of 1, same pragmas,
	 * a real library comparison — while `sqlite` holds the Rust stack. drizzle-rs-on-rusqlite versus
	 * drizzle-orm-on-Bun is a *stack* comparison however it is arranged.
	 *
	 * Absent on artifacts published before the field existed, where the database was the comparison
	 * group; `targetFamily` treats it that way rather than guessing.
	 */
	family?: string;
}

export interface ContractMeta {
	ver: string;
}

export interface QueryDoc {
	id: string;
	name: string;
	method: string;
	path: string;
	mix: number;
	params: string[];
	sql: QueryShape[];
}

export interface QueryShape {
	dialect: string;
	text: string;
}

export interface Summary {
	version: string;
	run_id: string;
	suite: string;
	target_id: string;
	group?: string;
	primary: Primary;
	spread: Spread;
	/**
	 * Either the saturation suite's result, or the legacy knee heuristic that shipped under the same
	 * key, or nothing at all. `#lib/saturation` is the only place allowed to tell them apart — see
	 * `readSaturation`, which treats everything but a real `outcome` as "not measured".
	 */
	saturation?: SaturationDoc | LegacySaturation;
	/**
	 * Latency at the highest load the target demonstrably sustained.
	 *
	 * This — not `primary.latency` — is the figure that measures the target. `primary.latency` merges
	 * the raw samples of every counted hold plateau, and once the ramp pushes a target past its
	 * throughput ceiling every further second contributes `VUs / throughput` of pure queueing, so the
	 * whole-ramp percentile ranks targets by how far the ramp overshot them rather than by how fast
	 * they answer. `#lib/service-latency` is the only place allowed to read this.
	 *
	 * Absent on every run published before the runner emitted it, which is most of them.
	 */
	latency?: LatencyUnderLoad;
}

/** How a target's sustained-load reading turned out. */
export type LatencyOutcome =
	/** The reference rung held. `reference` carries the figure. */
	| 'measured'
	/** The rung above the floor already failed, so the floor cannot be corroborated. No figure. */
	| 'floor_above_knee'
	/** The floor breached the error budget, so its latency is survivor-biased. No figure. */
	| 'floor_disqualified';

export interface LatencyUnderLoad {
	/**
	 * How far below the scaled offered rate a rung may fall and still count as sustained.
	 *
	 * Empirically derived rather than chosen. It no longer decides *where* the figure is read — the
	 * reference rung is fixed — only whether that rung held at all, which it does by a wide margin
	 * on every measurement taken so far.
	 */
	tolerance: number;
	outcome: LatencyOutcome;
	/**
	 * The reading: the ladder's fixed reference rung, the same offered load for every target.
	 *
	 * Deliberately not each target's own last sustained rung. That put the reading at the knee — the
	 * steepest part of the curve — where a run-to-run wobble across the threshold moved the published
	 * figure by up to fifteenfold, and it compared one target's p95 at 800 concurrent against
	 * another's at 100 in the same column, which is not an ordering however quiet the measurement is.
	 * How far a target got up the ramp is carried by `curve` instead.
	 *
	 * Absent in both `floor_*` outcomes, where no rung was corroborated.
	 */
	reference?: SustainedStep;
	/** Every rung, held or not, so the headline can be checked against its own working. */
	curve: SustainedStep[];
}

export interface SustainedStep {
	concurrency: number;
	rps: number;
	/** What a target holding its floor latency would have served here, by Little's law. */
	offered_rps: number;
	/** `rps / offered_rps`. At or above `1 - tolerance` the rung counts as sustained. */
	retention: number;
	latency: StepLatency;
	cpu: number;
	err: number;
	sustained: boolean;
	/** Why this rung was thrown out, or null. Currently only ever the error budget. */
	disqualified: string | null;
}

export interface SummaryResult extends Summary {
	cohort_id: string;
	target_key: string;
	target_name: string;
	target_description?: string;
	target_meta: TargetMeta;
	runner_os: string;
	runner_class: string;
	runner_label: string;
	/** CPU brand string of the machine this row was measured on. */
	runner_cpu: string;
	/** Logical cores on that machine. */
	runner_cores: number;
	/**
	 * The cpuset split in force for this row (`load=0-1 server=2 db=3`), or null when nothing was
	 * pinned. The ranking is scoped per OS, and this is how a reader tells a linux ranking whose
	 * families were isolated from each other from a macOS one where they could not be.
	 */
	runner_pinning: string | null;
}

export interface Primary {
	rps: AvgPeak;
	latency: LatencyPercentiles;
	cpu: AvgPeak;
	mem?: AvgPeak;
	err: number;
}

export interface AvgPeak {
	avg: number;
	peak: number;
}

/**
 * Latency percentiles in milliseconds.
 *
 * `p50` and `p90` are optional: runs published before the runner started measuring real
 * percentiles either omit them or (for older `p90`) carry an interpolated value. The
 * dashboard treats a present `p50` as the marker that percentiles are measured, and hides
 * `p90` otherwise rather than rendering a fabricated number.
 */
export interface LatencyPercentiles {
	avg: number;
	p50?: number;
	p90?: number;
	p95: number;
	p99: number;
	p999: number;
}

export interface Spread {
	trials: number;
	aggregate: string;
	rps: MinMax;
	p95: MinMax;
	variance: Variance;
	boxplot?: BoxPlot;
	ci95?: { rps?: MinMax; p95?: MinMax };
}

export interface MinMax {
	min: number;
	max: number;
}

export interface Variance {
	rps: VarianceMetric;
	p95: VarianceMetric;
	cpu: VarianceMetric;
	mem?: VarianceMetric;
	err: VarianceMetric;
}

export interface VarianceMetric {
	value: number;
	stdev: number;
	samples: number;
}

export interface BoxPlot {
	rps: BoxMetric;
	p95: BoxMetric;
	cpu: BoxMetric;
	mem?: BoxMetric;
	err: BoxMetric;
}

export interface BoxMetric {
	min: number;
	q1: number;
	median: number;
	q3: number;
	max: number;
	samples: number;
}

/**
 * What shipped under `summary.saturation` before the saturation suite existed.
 *
 * It is a p95-doubling heuristic over the *paced* run's hold buckets, and when it finds no knee it
 * falls back to the highest-throughput bucket — so a number is always produced whether or not
 * anything was measured. It is not a capacity figure and this dashboard never renders it: a paced
 * number wearing the word "saturation" is exactly the confusion the new vocabulary exists to end.
 * The type is kept so the discriminator below is exhaustive rather than a cast.
 */
export interface LegacySaturation {
	knee_rps: number;
	knee_p95: number;
}

/**
 * The saturation suite's result for one target.
 *
 * Unpaced closed loop, stepped concurrency: with no think time, N virtual users are N in-flight
 * requests, so the ramp is over concurrency and the headline is the highest step that held the SLO.
 * Every "we could not measure it" is a named outcome carrying no number, never a degraded value.
 */
export type SaturationDoc = SaturatedDoc | DidNotSaturateDoc | SloNeverMetDoc;

export interface SaturationSlo {
	/** Which percentile the objective is stated on, e.g. `p99`. */
	metric: string;
	ms: number;
}

interface SaturationBase {
	slo: SaturationSlo;
	/** Every measured step, including the ones that breached the SLO or were disqualified. */
	curve: SaturationStep[];
}

/** A peak was found: a step held the SLO and a later step breached it. The normal case. */
export interface SaturatedDoc extends SaturationBase {
	outcome: 'saturated';
	peak: SaturationPeak;
}

/**
 * Every step held the SLO, so the ramp never found the knee. The top step is a LOWER BOUND and is
 * never presented as a peak — the artifact deliberately carries no `peak` object.
 */
export interface DidNotSaturateDoc extends SaturationBase {
	outcome: 'did_not_saturate';
	lower_bound_rps: number;
}

/** Even the smallest step breached the SLO. There is no peak and no substitute for one. */
export interface SloNeverMetDoc extends SaturationBase {
	outcome: 'slo_never_met';
}

export interface SaturationPeak {
	concurrency: number;
	rps: number;
	latency: StepLatency;
	cpu: number;
	err: number;
}

export interface StepLatency {
	p50: number;
	p90: number;
	p95: number;
	p99: number;
}

export interface SaturationStep {
	concurrency: number;
	rps: number;
	latency: StepLatency;
	err: number;
	cpu: number;
	slo_met: boolean;
	/**
	 * Why this step cannot be the peak, or `null` when nothing disqualified it. A disqualified step
	 * is real data with a reason attached: it is still drawn, still hoverable, and still in the
	 * step table — it simply can never become the headline.
	 */
	disqualified: string | null;
}

export interface Timeseries {
	version: string;
	run_id: string;
	suite: string;
	target_id: string;
	/** Sample bucket width. Canonical since the runner renamed it. */
	bucket_s?: number;
	/** Deprecated alias for `bucket_s`, carrying the same value on artifacts that predate it. */
	interval_s?: number;
	points: TimeseriesPoint[];
}

/** Bucket width in seconds, preferring the canonical key over the deprecated alias. */
export function bucketSeconds(timeseries: Timeseries): number | null {
	return timeseries.bucket_s ?? timeseries.interval_s ?? null;
}

export type LoadPhase = 'warmup' | 'ramp' | 'hold';

export interface TimeseriesPoint {
	time: string;
	rps: number;
	err: number;
	latency: { avg: number; p50?: number; p90?: number; p95: number; p99: number; p999?: number };
	cpu: number[];
	mem_mb?: number;
	queries?: QueryTimeseriesPoint[];
	/** Optional additive fields; absent on artifacts produced before phase tracking. */
	trial?: number;
	stage?: number;
	phase?: LoadPhase;
	requests?: number;
	/** Virtual users offering load in this bucket, which is the x axis of the load replay. */
	vus?: number;
}

export interface QueryTimeseriesPoint {
	method: string;
	path: string;
	rps: number;
	err: number;
	latency: { avg: number; p95: number; p99: number; p999?: number };
}

export interface CompareItem {
	target_key: string;
	target_id: string;
	target_name: string;
	group?: string;
	runner_os?: string;
	base_value: number;
	head_value: number;
	delta: number;
	delta_pct: number;
}

export interface TargetCompareValue {
	key: string;
	label: string;
	value: number;
}

export interface TargetCompareVariance {
	label: string;
	value: number;
	stdev: number;
	samples: number;
}

/**
 * What the artifact actually recorded about trial-to-trial spread.
 * - `boxplot`: real quartiles from the runner.
 * - `range`: only min/max across trials plus the cross-trial median — no quartiles.
 * - `none`: no per-trial spread was recorded; only the aggregate value exists.
 */
export type SpreadKind = 'boxplot' | 'range' | 'none';

export interface SpreadDatum {
	spread: SpreadKind;
	min: number;
	max: number;
	/** Cross-trial median. Null only when the artifact has no aggregate for the metric. */
	median: number | null;
	/** Real quartiles; null unless `spread === 'boxplot'`. */
	q1: number | null;
	q3: number | null;
	samples: number;
}

export interface TargetCompareBox extends SpreadDatum {
	label: string;
}

export interface TargetCompareItem {
	target_key: string;
	target_id: string;
	target_name: string;
	target_description?: string;
	target_meta: TargetMeta;
	group?: string;
	run_id: string;
	runner_os: string;
	values: TargetCompareValue[];
	sort_value: number;
	variance: TargetCompareVariance;
	box: TargetCompareBox;
	err: number;
}

export interface TargetOption {
	key: string;
	label: string;
	target_id: string;
	target_name: string;
	target_meta: TargetMeta;
	runner_os: string;
}

export interface TrendPoint {
	cohort_id: string;
	run_id: string;
	start: string;
	git: string;
	rps_avg: number;
	rps_peak: number;
	latency_p95: number;
	latency_p99: number;
	cpu_avg: number;
	mem_avg?: number;
	mem_peak?: number;
	err: number;
}
