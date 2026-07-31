import type { TimeseriesPoint } from '$lib/types';
import { METRICS, type MetricKey } from '$lib/metrics';
import { meanCpu, representativeTrial, timeGapIndices, trialSampleText } from '$lib/trials';

export type SparkLineMetric = MetricKey;

export interface SparkLineProps {
	points: TimeseriesPoint[];
	metric: SparkLineMetric;
	trialCount: number;
}

/** One sample bucket. `value === null` marks a break so the line does not span a real gap. */
export interface SparkPoint {
	index: number;
	value: number | null;
}

function metricValue(point: TimeseriesPoint, metric: SparkLineMetric): number {
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

/**
 * Sparkline data for one target.
 *
 * All geometry — scales, paths, the area under the line — belongs to LayerChart. What is left
 * here is the part that is about benchmarks rather than drawing: which trial to chart, where the
 * gaps between buckets are, and how to describe the sample to the reader.
 */
export class SparkLineState {
	#props: () => SparkLineProps;

	constructor(props: () => SparkLineProps) {
		this.#props = props;
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

	/**
	 * The trial is chosen once, by throughput, and reused for every metric tab so switching tabs
	 * never switches which trial is on screen.
	 */
	trial = $derived(representativeTrial(this.points, this.trialCount));

	series: SparkPoint[] = $derived.by(() => {
		const points = this.trial.points;
		// A representative trial should be contiguous, but keep breaking on real gaps in case the
		// artifact interleaves buckets.
		const gaps = timeGapIndices(points);
		const metric = this.metric;
		const series: SparkPoint[] = [];

		points.forEach((point, index) => {
			if (gaps.has(index) && series.length > 0) {
				series.push({ index: index - 0.5, value: null });
			}
			series.push({ index, value: metricValue(point, metric) });
		});

		return series;
	});

	hasSeries = $derived(this.series.some((point) => point.value !== null));

	latestValue = $derived.by(() => {
		for (let index = this.series.length - 1; index >= 0; index -= 1) {
			const value = this.series[index].value;
			if (value !== null && Number.isFinite(value)) return value;
		}
		return null;
	});

	valueText = $derived.by(() => {
		const value = this.latestValue;
		return value === null ? 'no samples' : METRICS[this.metric].format(value);
	});

	metricLabel = $derived(METRICS[this.metric].label);
	sampleText = $derived(trialSampleText(this.trial, this.trialCount));
}
