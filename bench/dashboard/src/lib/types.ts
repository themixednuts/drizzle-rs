export interface RunIndex {
	version: string;
	runs: RunIndexEntry[];
}

export interface RunIndexEntry {
	run_id: string;
	suite: string;
	status: string;
	class: string;
	git: string;
	start: string;
	end: string;
	targets: string[];
}

export interface Manifest {
	version: string;
	run_id: string;
	suite: string;
	git: string;
	workload: string;
	targets: string[];
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
	compat?: {
		workload: string;
		class: string;
		targets: string[];
	};
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
	ci95?: { rps?: MinMax; p95?: MinMax };
}

export interface MinMax {
	min: number;
	max: number;
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
}

export interface CompareItem {
	target_id: string;
	base_value: number;
	head_value: number;
	delta: number;
	delta_pct: number;
}

export interface TrendPoint {
	run_id: string;
	start: string;
	git: string;
	rps_avg: number;
	rps_peak: number;
	latency_p95: number;
	latency_p99: number;
	cpu_avg: number;
	err: number;
}
