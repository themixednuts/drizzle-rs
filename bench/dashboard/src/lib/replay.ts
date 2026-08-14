import type { TimeseriesPoint } from './types';

/**
 * The load test as something to play back rather than a number to read.
 *
 * Every other view on this site reports what a run ended on. This one keeps the run: virtual users
 * climb along the bottom, each target's rate draws itself across, and a readout counts along with
 * it. Watching one line flatten while another keeps climbing says where a target ran out of
 * headroom, which no summary figure can.
 *
 * The reduction here is deliberate and is the whole reason this module exists rather than the chart
 * plotting raw buckets. The runner samples every second and concatenates all five trials into one
 * series, so plotting it as recorded shows the same ramp five times over, and each hold stage
 * appears as fifteen buckets at the same virtual-user count with the rate jittering between them —
 * a vertical scribble at every plateau, which reads as instability rather than as a steady state.
 * One trial, one median point per load level, is the line the ramp actually traced.
 */

export interface ReplayPoint {
	vus: number;
	rps: number;
	p95: number;
	err: number;
	cpu: number;
}

export interface ReplaySeries {
	targetId: string;
	name: string;
	points: ReplayPoint[];
	/**
	 * What this series belongs to, so a caller can narrow the set without re-reading the artifacts.
	 *
	 * The ranking loads every target in a set once and then shows whichever ones its filters select,
	 * which is what makes the replay's picker the same field as the table's rows. A run's own page
	 * shows all of them and leaves these unset.
	 */
	os?: string;
	db?: string;
	/** Median request rate, used to order the series so the fastest are the ones drawn first. */
	rps?: number;
}

export interface ReplayView {
	series: ReplaySeries[];
	/** The highest load level reached, which is the x axis's ceiling. */
	maxVus: number;
}

/** One target's recorded buckets, before reduction. */
export interface ReplayInput {
	targetId: string;
	name: string;
	points: readonly TimeseriesPoint[];
	os?: string;
	db?: string;
	rps?: number;
}

/** Mean across cores, since the artifact records one sample per core. */
function meanCpu(cpu: readonly number[] | undefined): number {
	if (!cpu || cpu.length === 0) return 0;
	return cpu.reduce((total, value) => total + value, 0) / cpu.length;
}

/** One trial's ramp, not all five concatenated. */
function firstTrial(points: readonly TimeseriesPoint[]): TimeseriesPoint[] {
	const hasTrials = points.some((point) => point.trial !== undefined);
	if (!hasTrials) return [...points];
	const first = Math.min(...points.map((point) => point.trial ?? 0));
	return points.filter((point) => (point.trial ?? 0) === first);
}

/** Median, so one loud bucket inside a hold stage does not define the level. */
function median(values: number[]): number {
	if (values.length === 0) return 0;
	const sorted = [...values].sort((a, b) => a - b);
	const middle = Math.floor(sorted.length / 2);
	return sorted.length % 2 === 0 ? (sorted[middle - 1] + sorted[middle]) / 2 : sorted[middle];
}

/** Collapse the buckets to one point per virtual-user level. */
function byVuLevel(points: readonly TimeseriesPoint[]): ReplayPoint[] {
	const levels = new Map<number, TimeseriesPoint[]>();
	for (const point of points) {
		const vus = point.vus ?? 0;
		levels.set(vus, [...(levels.get(vus) ?? []), point]);
	}

	return [...levels.entries()]
		.sort((a, b) => a[0] - b[0])
		.map(([vus, bucket]) => ({
			vus,
			rps: median(bucket.map((point) => point.rps)),
			p95: median(bucket.map((point) => point.latency.p95)),
			err: median(bucket.map((point) => point.err)),
			cpu: median(bucket.map((point) => meanCpu(point.cpu))),
		}));
}

/**
 * Build the replay, or `null` when this run cannot be replayed.
 *
 * Null is the answer whenever the artifacts carry no virtual-user count — an older runner, or a
 * suite that offers a fixed load and never ramps. Every level would collapse to zero and the chart
 * would draw one point per target while looking exactly like a ramp that went nowhere. A caller
 * that gets null should draw something else rather than an empty playback.
 */
export function buildReplay(inputs: readonly ReplayInput[]): ReplayView | null {
	const series = inputs
		.map((input) => ({
			targetId: input.targetId,
			name: input.name,
			os: input.os,
			db: input.db,
			rps: input.rps,
			points: byVuLevel(firstTrial(input.points)),
		}))
		.filter((entry) => entry.points.length > 1);

	if (series.length === 0) return null;

	const maxVus = Math.max(...series.flatMap((entry) => entry.points.map((point) => point.vus)));
	if (maxVus <= 0) return null;

	return { series, maxVus };
}
