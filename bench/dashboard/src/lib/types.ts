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
