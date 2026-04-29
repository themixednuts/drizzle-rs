export interface RunIndex {
	version: string;
	runs: RunIndexEntry[];
}

export interface RunIndexEntry {
	run_id: string;
	name: string;
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
	name: string;
	suite: string;
	git: string;
	workload: string;
	targets: string[];
	target_meta: TargetMeta[];
	queries: QueryDoc[];
	start: string;
	end: string;
	status: string;
	seed: number;
	load: {
		executor: string;
		stages: number;
		duration_s: number;
		max_vus: number;
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
		headroom: { cpu_peak: number; net_peak: number };
	};
	trials: { count: number; aggregate: string };
}

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
	saturation: Saturation;
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

export interface LatencyPercentiles {
	avg: number;
	p90: number;
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

export interface Saturation {
	knee_rps: number;
	knee_p95: number;
}

export interface Timeseries {
	version: string;
	run_id: string;
	suite: string;
	target_id: string;
	interval_s: number;
	points: TimeseriesPoint[];
}

export interface TimeseriesPoint {
	time: string;
	rps: number;
	err: number;
	latency: { avg: number; p95: number; p99: number; p999?: number };
	cpu: number[];
	mem_mb?: number;
	queries?: QueryTimeseriesPoint[];
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

export interface TargetCompareItem {
	target_key: string;
	target_id: string;
	target_name: string;
	target_description?: string;
	target_meta: TargetMeta;
	group?: string;
	runner_os: string;
	values: TargetCompareValue[];
	sort_value: number;
	variance: TargetCompareVariance;
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
