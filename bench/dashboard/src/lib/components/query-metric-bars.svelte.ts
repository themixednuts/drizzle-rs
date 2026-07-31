import type { QueryDoc, QueryTimeseriesPoint, TimeseriesPoint } from '$lib/types';
import { METRICS } from '$lib/metrics';
import { representativeTrial, timeGapIndices, trialSampleText } from '$lib/trials';
import type { SparkLineMetric, SparkPoint } from './spark-line.svelte';

export interface QueryMetricBarsProps {
	queries: QueryDoc[];
	points: TimeseriesPoint[];
	metric: SparkLineMetric;
	trialCount: number;
}

export interface QueryMetricRow {
	query: QueryDoc;
	series: SparkPoint[];
	hasSamples: boolean;
	latest: number;
	avg: number;
	peak: number;
}

function metricValue(query: QueryTimeseriesPoint, metric: SparkLineMetric): number | null {
	if (metric === 'rps') return query.rps;
	if (metric === 'latency') return query.latency.p95;
	return null;
}

function mean(values: number[]): number {
	return values.length === 0 ? 0 : values.reduce((sum, value) => sum + value, 0) / values.length;
}

/**
 * Route-level breakdown of the metric the target's sparkline is showing.
 *
 * CPU and memory are sampled at the process level, so they are deliberately not attributed to
 * routes — `isAttributable` is what the component reads to say so rather than drawing flat zeros.
 */
export class QueryMetricBarsState {
	#props: () => QueryMetricBarsProps;

	constructor(props: () => QueryMetricBarsProps) {
		this.#props = props;
	}

	get queries(): QueryDoc[] {
		return this.#props().queries;
	}

	get points(): TimeseriesPoint[] {
		return this.#props().points;
	}

	get metric(): SparkLineMetric {
		return this.#props().metric;
	}

	get trialCount(): number {
		return Math.max(1, Math.floor(this.#props().trialCount));
	}

	/** Same trial the sparkline charts, so the two graphs always describe the same window. */
	trial = $derived(representativeTrial(this.points, this.trialCount));

	isAttributable = $derived(this.metric === 'rps' || this.metric === 'latency');

	hasQueryMetrics = $derived(this.points.some((point) => (point.queries?.length ?? 0) > 0));

	/** One shared y-scale across routes, so a busy route visibly dwarfs a quiet one. */
	maxValue = $derived.by(() => {
		if (!this.isAttributable) return 1;
		const metric = this.metric;
		const values = this.trial.points.flatMap((point) =>
			(point.queries ?? [])
				.map((query) => metricValue(query, metric))
				.filter((value): value is number => value !== null),
		);
		return Math.max(1, ...values);
	});

	rows: QueryMetricRow[] = $derived.by(() => {
		if (!this.isAttributable) return [];

		const metric = this.metric;
		const points = this.trial.points;
		const gaps = timeGapIndices(points);

		return this.queries.map((query) => {
			const series: SparkPoint[] = [];
			const observed: number[] = [];

			points.forEach((point, index) => {
				const sample = point.queries?.find(
					(item) => item.method === query.method && item.path === query.path,
				);
				const value = sample ? (metricValue(sample, metric) ?? 0) : 0;
				if (sample) observed.push(value);
				if (gaps.has(index) && series.length > 0) {
					series.push({ index: index - 0.5, value: null });
				}
				series.push({ index, value });
			});

			return {
				query,
				series,
				hasSamples: observed.length > 0,
				latest: observed.at(-1) ?? 0,
				avg: mean(observed),
				peak: observed.reduce((max, value) => Math.max(max, value), 0),
			};
		});
	});

	sampleText = $derived(trialSampleText(this.trial, this.trialCount));
	metricLabel = $derived(this.metric === 'rps' ? 'route rps' : 'route p95');

	unavailableText = $derived.by(() => {
		if (!this.isAttributable) {
			return 'CPU and memory are sampled at the target process level, so the runner does not attribute them to individual routes.';
		}
		return 'This run artifact does not include route-level metric buckets. Re-run with the current runner to populate per-query graphs.';
	});

	format(value: number): string {
		return METRICS[this.metric].format(value);
	}
}
