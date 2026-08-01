import { fmtLatency, fmtRps } from './format';
import { segmentTrials } from './trials';
import type { TimeseriesPoint } from './types';

/**
 * The run-detail centrepiece: every target on one chart, second by second, with a dashed rule
 * wherever a new trial starts.
 *
 * This is a different reading than the per-target sparkline beside it. The sparkline shows one
 * representative trial so a single number has a shape you can trust; this shows *all* trials for
 * *all* targets at once, which is the only view where run-to-run variance and the gap between two
 * libraries are visible in the same glance.
 *
 * Derived on the server for the same reason the sparklines are: the raw artifacts are ~800 KB per
 * target and what a chart needs is a few hundred numbers.
 */

export type OverlayMetric = 'rps' | 'latency';

export interface OverlaySeries {
	targetId: string;
	name: string;
	/** drizzle-rs takes the accent; everything else steps down a neutral ramp. */
	isOurs: boolean;
	/** 0-based position in the neutral ramp, for targets that are not ours. */
	tone: number;
	/** Index-aligned values. `null` is a bucket this target did not record. */
	values: (number | null)[];
}

export interface OverlayChart {
	metric: OverlayMetric;
	title: string;
	series: OverlaySeries[];
	/** x indices where a trial boundary falls, for the dashed verticals. */
	breaks: number[];
	/** Label and x position for each trial, drawn under the plot. */
	trials: { label: string; start: number; end: number }[];
	length: number;
	max: number;
	/** Formatted y-axis labels: top, middle, and an implicit zero. */
	yTop: string;
	yMid: string;
	/** Set when there is nothing honest to draw; the component states this instead. */
	unavailable: string | null;
}

export interface OverlayTarget {
	targetId: string;
	name: string;
	isOurs: boolean;
	points: readonly TimeseriesPoint[];
}

function value(point: TimeseriesPoint, metric: OverlayMetric): number {
	const raw = metric === 'rps' ? point.rps : point.latency.p95;
	return Number.isFinite(raw) ? Math.round(raw * 100) / 100 : 0;
}

const TITLES: Record<OverlayMetric, string> = {
	rps: 'Requests per second, second by second',
	latency: 'p95 latency, second by second',
};

export function buildOverlay(
	targets: readonly OverlayTarget[],
	metric: OverlayMetric,
	trialCount: number,
): OverlayChart {
	const empty: OverlayChart = {
		metric,
		title: TITLES[metric],
		series: [],
		breaks: [],
		trials: [],
		length: 0,
		max: 1,
		yTop: '',
		yMid: '',
		unavailable: 'This run published no per-second timeseries.',
	};

	const withPoints = targets.filter((target) => target.points.length > 0);
	if (withPoints.length === 0) return empty;

	// Every target ran the same load profile, but nothing guarantees they recorded the same number
	// of buckets. Segmenting each independently and then padding each trial to the widest one keeps
	// the trial boundaries aligned across targets without inventing samples: a target that recorded
	// fewer buckets in a trial simply has nulls there, and its line breaks.
	const segmented = withPoints.map((target) => ({
		target,
		slices: segmentTrials(target.points, trialCount).slices,
	}));

	const trialTotal = Math.max(...segmented.map((entry) => entry.slices.length));
	const widths: number[] = [];
	for (let trial = 0; trial < trialTotal; trial += 1) {
		widths.push(Math.max(...segmented.map((entry) => entry.slices[trial]?.length ?? 0)));
	}

	const trials: OverlayChart['trials'] = [];
	const breaks: number[] = [];
	let cursor = 0;
	widths.forEach((width, index) => {
		if (index > 0) breaks.push(cursor);
		trials.push({
			label: index === 0 ? 'trial 1' : String(index + 1),
			start: cursor,
			end: cursor + width,
		});
		cursor += width;
	});
	const length = cursor;
	if (length === 0) return empty;

	let max = 0;
	const series: OverlaySeries[] = segmented.map((entry, index) => {
		const values: (number | null)[] = [];
		widths.forEach((width, trial) => {
			const slice = entry.slices[trial] ?? [];
			for (let position = 0; position < width; position += 1) {
				const point = slice[position];
				if (point) {
					const next = value(point, metric);
					if (next > max) max = next;
					values.push(next);
				} else {
					values.push(null);
				}
			}
		});
		return {
			targetId: entry.target.targetId,
			name: entry.target.name,
			isOurs: entry.target.isOurs,
			tone: index % 3,
			values,
		};
	});

	const ceiling = max > 0 ? max : 1;
	const format = metric === 'rps' ? fmtRps : fmtLatency;

	return {
		metric,
		title: TITLES[metric],
		series,
		breaks,
		trials: trialTotal > 1 ? trials : [],
		length,
		max: ceiling,
		yTop: format(ceiling),
		yMid: format(ceiling / 2),
		unavailable: null,
	};
}
