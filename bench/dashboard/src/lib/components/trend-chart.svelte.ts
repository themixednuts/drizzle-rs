import type { TrendPoint } from '#lib/types';
import type { MetricKey } from '#lib/metrics';

export type TrendChartMetric =
	| 'rps_avg'
	| 'rps_peak'
	| 'latency_p95'
	| 'latency_p99'
	| 'cpu_avg'
	| 'mem_avg'
	| 'mem_peak'
	| 'err';

export interface TrendChartProps {
	points: TrendPoint[];
	field: TrendChartMetric;
	metric: MetricKey;
}

export interface TrendSample {
	index: number;
	/** `null` where this cohort published no value for the field, which breaks the line there. */
	value: number | null;
	runId: string;
	git: string;
}

/** How many commit labels the x axis can carry before they collide. */
const MAX_X_TICKS = 8;

/**
 * Trend data for one target across benchmark sets.
 *
 * Scales, ticks, paths and hit-testing are LayerChart's. This class only decides which cohorts
 * have a value for the field — cohorts that do not become `null` so the line breaks rather than
 * drawing straight through missing history — and which commits get an axis label.
 */
export class TrendChartState {
	#props: () => TrendChartProps;

	constructor(props: () => TrendChartProps) {
		this.#props = props;
	}

	get points(): TrendPoint[] {
		return this.#props().points;
	}

	get field(): TrendChartMetric {
		return this.#props().field;
	}

	get metric(): MetricKey {
		return this.#props().metric;
	}

	series: TrendSample[] = $derived.by(() => {
		const field = this.field;
		return this.points.map((point, index) => {
			const raw = point[field];
			return {
				index,
				value: typeof raw === 'number' && Number.isFinite(raw) ? raw : null,
				runId: point.run_id,
				git: point.git,
			};
		});
	});

	hasData = $derived(this.series.some((sample) => sample.value !== null));

	xTicks: number[] = $derived.by(() => {
		const total = this.series.length;
		if (total === 0) return [];
		const step = Math.max(1, Math.ceil(total / MAX_X_TICKS));
		const ticks = this.series
			.filter((_, index) => index % step === 0)
			.map((sample) => sample.index);
		const last = total - 1;
		if (ticks.at(-1) !== last) ticks.push(last);
		return ticks;
	});

	gitAt(index: number): string {
		return this.series[index]?.git ?? '';
	}
}
