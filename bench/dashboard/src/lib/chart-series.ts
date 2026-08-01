import { METRICS, type MetricKey } from './metrics';
import { meanCpu, representativeTrial, timeGapIndices, trialSampleText } from './trials';
import type { QueryDoc, QueryTimeseriesPoint, TimeseriesPoint } from './types';

/**
 * Turning a target's raw timeseries into something a chart can draw.
 *
 * This runs on the server, not in the component, for one measured reason: a single target's
 * `timeseries.json` is ~800 KB, and a run has several targets. Shipping those to the browser so
 * the client could derive a few hundred plotted points would mean a multi-megabyte HTML payload
 * and a chart that only appears after hydration. Deriving here means the server renders the real
 * chart into the HTML and sends ~1% of the data.
 *
 * Nothing in this file touches Svelte or the DOM, so the same code backs the initial server render
 * and the remote function that swaps metrics client-side — the two can never disagree.
 */

/**
 * A plotted series, as bare values indexed by position. `null` is a deliberate gap, so the line
 * breaks instead of spanning it.
 *
 * Bare numbers rather than `{ index, value }` objects because this is the largest thing the page
 * serializes: the property names cost more than the data. The component pairs each value with its
 * index on the way into the chart.
 */
export type Series = (number | null)[];

/** What a chart component feeds LayerChart, rebuilt from `Series` at render time. */
export interface SeriesPoint {
	index: number;
	value: number | null;
}

export function toPoints(series: Series): SeriesPoint[] {
	return series.map((value, index) => ({ index, value }));
}

export interface RouteSeries {
	id: string;
	name: string;
	method: string;
	path: string;
	series: Series;
	hasSamples: boolean;
	latest: number;
	avg: number;
	peak: number;
}

export interface TargetChart {
	metric: MetricKey;
	/** Short label for the metric, e.g. `p95`. */
	label: string;
	series: Series;
	hasSeries: boolean;
	/** Formatted latest value, or a plain statement that there were no samples. */
	valueText: string;
	/** Provenance: how many buckets, and which trial they came from. */
	sampleText: string;
	/** `null` when the metric is not attributable to individual routes (cpu, mem). */
	routes: RouteSeries[] | null;
	/** Why `routes` is null or empty; `null` when routes are present. */
	routesUnavailable: string | null;
	/** Shared y-domain across routes, so a busy route visibly dwarfs a quiet one. */
	routeMax: number;
}

/** Trim float noise so the serialized payload stays small without changing what is drawn. */
function round(value: number): number {
	return Number.isFinite(value) ? Math.round(value * 100) / 100 : 0;
}

function metricValue(point: TimeseriesPoint, metric: MetricKey): number {
	switch (metric) {
		case 'rps':
			return point.rps;
		case 'latency':
			return point.latency.p95;
		case 'cpu':
			return meanCpu(point);
		case 'mem':
			return point.mem_mb ?? 0;
		case 'err':
			return point.err;
	}
}

function routeValue(query: QueryTimeseriesPoint, metric: MetricKey): number | null {
	if (metric === 'rps') return query.rps;
	if (metric === 'latency') return query.latency.p95;
	return null;
}

/** Route-level buckets only exist for metrics the runner attributes per request. */
function isAttributable(metric: MetricKey): boolean {
	return metric === 'rps' || metric === 'latency';
}

function mean(values: number[]): number {
	return values.length === 0 ? 0 : values.reduce((sum, value) => sum + value, 0) / values.length;
}

export function buildTargetChart(
	points: readonly TimeseriesPoint[],
	queries: readonly QueryDoc[],
	metric: MetricKey,
	trialCount: number,
): TargetChart {
	const definition = METRICS[metric];
	const trials = Math.max(1, Math.floor(trialCount));

	// The trial is chosen once, by throughput, and reused for every metric so switching metrics
	// never silently switches which trial is on screen.
	const trial = representativeTrial(points, trials);
	const chartPoints = trial.points;
	// A representative trial should be contiguous, but keep breaking on real gaps in case the
	// artifact interleaves buckets.
	const gaps = timeGapIndices(chartPoints);

	const series: Series = [];
	chartPoints.forEach((point, index) => {
		if (gaps.has(index) && series.length > 0) series.push(null);
		series.push(round(metricValue(point, metric)));
	});

	let latest: number | null = null;
	for (let index = series.length - 1; index >= 0; index -= 1) {
		const value = series[index];
		if (value !== null) {
			latest = value;
			break;
		}
	}

	const attributable = isAttributable(metric);
	const hasRouteBuckets = points.some((point) => (point.queries?.length ?? 0) > 0);

	let routes: RouteSeries[] | null = null;
	let routeMax = 1;
	let routesUnavailable: string | null = null;

	if (!attributable) {
		routesUnavailable =
			'CPU and memory are sampled at the target process level, so the runner does not attribute them to individual routes.';
	} else if (!hasRouteBuckets) {
		routesUnavailable =
			'This run artifact does not include route-level metric buckets. Re-run with the current runner to populate per-query graphs.';
	} else {
		const observedAll = chartPoints.flatMap((point) =>
			(point.queries ?? [])
				.map((query) => routeValue(query, metric))
				.filter((value): value is number => value !== null),
		);
		routeMax = Math.max(1, ...observedAll);

		routes = queries.map((query) => {
			const routeSeries: Series = [];
			const observed: number[] = [];

			chartPoints.forEach((point, index) => {
				const sample = point.queries?.find(
					(item) => item.method === query.method && item.path === query.path,
				);
				const value = sample ? (routeValue(sample, metric) ?? 0) : 0;
				if (sample) observed.push(value);
				if (gaps.has(index) && routeSeries.length > 0) routeSeries.push(null);
				routeSeries.push(round(value));
			});

			return {
				id: query.id,
				name: query.name,
				method: query.method,
				path: query.path,
				series: routeSeries,
				hasSamples: observed.length > 0,
				latest: round(observed.at(-1) ?? 0),
				avg: round(mean(observed)),
				peak: round(observed.reduce((max, value) => Math.max(max, value), 0)),
			};
		});
	}

	return {
		metric,
		label: definition.label,
		series,
		hasSeries: latest !== null,
		valueText: latest === null ? 'no samples' : definition.format(latest),
		sampleText: trialSampleText(trial, trials),
		routes,
		routesUnavailable,
		routeMax,
	};
}

/** An honest empty chart for a target whose timeseries artifact is missing entirely. */
export function emptyTargetChart(metric: MetricKey): TargetChart {
	return {
		metric,
		label: METRICS[metric].label,
		series: [],
		hasSeries: false,
		valueText: 'no samples',
		sampleText: 'no timeseries recorded',
		routes: null,
		routesUnavailable: 'This run published no timeseries for this target.',
		routeMax: 1,
	};
}
