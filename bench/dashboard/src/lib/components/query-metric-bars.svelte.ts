import type { QueryDoc, QueryTimeseriesPoint, TimeseriesPoint } from '$lib/types';
import { fmtLatency, fmtPct, fmtRps } from '$lib/format';
import type { SparkLineMetric } from './spark-line.svelte';

const W = 150;
const H = 28;
const PAD = 2;

export interface QueryMetricRow {
	query: QueryDoc;
	values: number[];
	latest: number;
	avg: number;
	peak: number;
	path: string;
	dot: { x: number; y: number } | null;
}

function metricValue(query: QueryTimeseriesPoint, metric: SparkLineMetric): number | null {
	if (metric === 'rps') return query.rps;
	if (metric === 'latency') return query.latency.p95;
	if (metric === 'cpu' || metric === 'mem') return null;
	return null;
}

function fmt(value: number, metric: SparkLineMetric): string {
	if (metric === 'rps') return fmtRps(value);
	if (metric === 'latency') return fmtLatency(value);
	if (metric === 'cpu') return fmtPct(value / 100);
	if (metric === 'mem') return `${value.toFixed(1)}MB`;
	return value.toFixed(2);
}

function avg(values: number[]): number {
	return values.length === 0 ? 0 : values.reduce((sum, value) => sum + value, 0) / values.length;
}

function peak(values: number[]): number {
	return values.reduce((max, value) => Math.max(max, value), 0);
}

function linePath(values: number[], max: number): { path: string; dot: QueryMetricRow['dot'] } {
	if (values.length === 0) return { path: '', dot: null };

	const range = max > 0 ? max : 1;
	if (values.length === 1) {
		return {
			path: '',
			dot: { x: W / 2, y: H - PAD - (values[0] / range) * (H - PAD * 2) }
		};
	}

	const step = (W - PAD * 2) / (values.length - 1);
	const coords = values.map((value, index) => ({
		x: PAD + index * step,
		y: H - PAD - (value / range) * (H - PAD * 2)
	}));
	return {
		path: coords
			.map(({ x, y }, index) => (index === 0 ? 'M' : 'L') + x.toFixed(1) + ',' + y.toFixed(1))
			.join(' '),
		dot: null
	};
}

export class QueryMetricBarsState {
	#queries: () => QueryDoc[];
	#points: () => TimeseriesPoint[];
	#metric: () => SparkLineMetric;

	rows = $derived.by(() => {
		const metric = this.metric;
		if (metric === 'cpu' || metric === 'mem') return [];

		const maxValue = this.maxValue;
		return this.queries.map((query) => {
			const values = this.points.map((point) => {
				const sample = point.queries?.find(
					(item) => item.method === query.method && item.path === query.path
				);
				return sample ? (metricValue(sample, metric) ?? 0) : 0;
			});
			const shape = linePath(values, maxValue);
			return {
				query,
				values,
				latest: values.at(-1) ?? 0,
				avg: avg(values),
				peak: peak(values),
				path: shape.path,
				dot: shape.dot
			};
		});
	});

	maxValue = $derived.by(() => {
		const metric = this.metric;
		if (metric === 'cpu' || metric === 'mem') return 1;
		const values = this.points.flatMap((point) =>
			(point.queries ?? [])
				.map((query) => metricValue(query, metric))
				.filter((value): value is number => value !== null)
		);
		return Math.max(1, ...values);
	});

	hasQueryMetrics = $derived(this.points.some((point) => (point.queries?.length ?? 0) > 0));
	isAttributable = $derived(this.metric === 'rps' || this.metric === 'latency');

	metricLabel = $derived.by(() => {
		if (this.metric === 'rps') return 'route rps';
		if (this.metric === 'latency') return 'route p95';
		if (this.metric === 'cpu') return 'cpu';
		return 'mem';
	});

	unavailableText = $derived.by(() => {
		if (!this.isAttributable) {
			return 'CPU and memory are sampled at the target process level, so the runner does not attribute them to individual routes.';
		}
		return 'This run artifact does not include route-level metric buckets. Re-run with the current runner to populate per-query graphs.';
	});

	constructor(
		queries: () => QueryDoc[],
		points: () => TimeseriesPoint[],
		metric: () => SparkLineMetric
	) {
		this.#queries = queries;
		this.#points = points;
		this.#metric = metric;
	}

	get queries() {
		return this.#queries();
	}

	get points() {
		return this.#points();
	}

	get metric() {
		return this.#metric();
	}

	format(value: number): string {
		return fmt(value, this.metric);
	}
}

export const QUERY_METRIC_VIEWBOX = { width: W, height: H };
