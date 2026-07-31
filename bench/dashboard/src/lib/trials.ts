import type { TimeseriesPoint } from './types';

/** Smallest inter-bucket gap that can mark a trial boundary, regardless of sample interval. */
const MIN_GAP_MS = 10_000;

/** How the buckets in a timeseries were split into trials. */
export type TrialSegmentationMethod =
	| 'trial-field' // points carry an explicit `trial` index
	| 'time-gap' // boundaries detected from gaps between bucket timestamps
	| 'even-split' // fallback: assume trials are equal-length
	| 'single'; // one trial, or too few buckets to split

export interface TrialSegmentation {
	slices: TimeseriesPoint[][];
	method: TrialSegmentationMethod;
}

export interface RepresentativeTrial {
	points: TimeseriesPoint[];
	/** 0-based index of the chosen trial, or null when the points were not split. */
	index: number | null;
	total: number;
	method: TrialSegmentationMethod;
}

/** Indices where a new segment starts because of a gap in bucket timestamps. */
export function timeGapIndices(points: readonly TimeseriesPoint[]): Set<number> {
	const gaps = new Set<number>();
	const times = points.map((point) => Date.parse(point.time));
	const deltas = times
		.slice(1)
		.map((time, index) => time - times[index])
		.filter((delta) => Number.isFinite(delta) && delta > 0)
		.sort((a, b) => a - b);
	if (deltas.length === 0) return gaps;

	const medianDelta = deltas[Math.floor(deltas.length / 2)] ?? 0;
	const threshold = Math.max(MIN_GAP_MS, medianDelta * 5);

	for (let index = 1; index < times.length; index += 1) {
		const delta = times[index] - times[index - 1];
		if (Number.isFinite(delta) && delta > threshold) gaps.add(index);
	}
	return gaps;
}

/**
 * Split a target's buckets into per-trial slices.
 *
 * Preference order: the explicit `trial` field on new artifacts, then real time gaps between
 * buckets, then an equal-length split as a last resort. The chosen method is returned so the
 * UI can say "all trials" instead of claiming a median trial it never actually isolated.
 */
export function segmentTrials(
	points: readonly TimeseriesPoint[],
	trialCount: number,
): TrialSegmentation {
	const all = [...points];
	if (all.length === 0) return { slices: [], method: 'single' };

	if (all.some((point) => typeof point.trial === 'number')) {
		const byTrial = new Map<number, TimeseriesPoint[]>();
		for (const point of all) {
			const trial = typeof point.trial === 'number' ? point.trial : -1;
			const bucket = byTrial.get(trial);
			if (bucket) bucket.push(point);
			else byTrial.set(trial, [point]);
		}
		const slices = [...byTrial.entries()].sort((a, b) => a[0] - b[0]).map(([, slice]) => slice);
		if (slices.length > 1) return { slices, method: 'trial-field' };
		return { slices: [all], method: 'single' };
	}

	const gaps = [...timeGapIndices(all)].sort((a, b) => a - b);
	if (gaps.length > 0) {
		const slices: TimeseriesPoint[][] = [];
		let start = 0;
		for (const gap of gaps) {
			slices.push(all.slice(start, gap));
			start = gap;
		}
		slices.push(all.slice(start));
		const nonEmpty = slices.filter((slice) => slice.length > 0);
		if (nonEmpty.length > 1) return { slices: nonEmpty, method: 'time-gap' };
	}

	const safeTrialCount = Math.max(1, Math.floor(trialCount));
	if (safeTrialCount > 1 && all.length >= safeTrialCount * 2) {
		const slices: TimeseriesPoint[][] = [];
		for (let trial = 0; trial < safeTrialCount; trial += 1) {
			const start = Math.round((all.length * trial) / safeTrialCount);
			const end = Math.round((all.length * (trial + 1)) / safeTrialCount);
			const slice = all.slice(start, end);
			if (slice.length > 0) slices.push(slice);
		}
		if (slices.length > 1) return { slices, method: 'even-split' };
	}

	return { slices: [all], method: 'single' };
}

/**
 * Pick the trial to chart.
 *
 * The choice is always ranked by throughput, never by the metric being displayed, so switching
 * the metric tab does not silently swap which trial you are looking at. When artifacts carry a
 * `phase`, only hold-phase buckets contribute to the ranking (warmup and ramp are excluded from
 * the runner's own aggregates for the same reason).
 */
export function representativeTrial(
	points: readonly TimeseriesPoint[],
	trialCount: number,
): RepresentativeTrial {
	const { slices, method } = segmentTrials(points, trialCount);
	if (slices.length <= 1) {
		return { points: slices[0] ?? [], index: null, total: slices.length, method };
	}

	const ranked = slices
		.map((slice, index) => ({ index, score: throughputScore(slice) }))
		.sort((a, b) => a.score - b.score);
	const chosen = ranked[Math.floor(ranked.length / 2)] ?? ranked[0];
	return {
		points: slices[chosen.index] ?? [],
		index: chosen.index,
		total: slices.length,
		method,
	};
}

/** Human-readable provenance for the charted buckets. */
export function trialSampleText(trial: RepresentativeTrial, trialCount: number): string {
	const count = trial.points.length;
	const buckets = `${count} bucket${count === 1 ? '' : 's'}`;
	if (trial.index === null) {
		// Segmentation bailed: the chart is every bucket, so do not claim a median trial.
		return trialCount > 1 ? `${buckets} / all ${trialCount} trials` : buckets;
	}
	return `${buckets} / median trial ${trial.index + 1} of ${trial.total}`;
}

function throughputScore(points: readonly TimeseriesPoint[]): number {
	const hold = points.filter((point) => point.phase === 'hold');
	const scored = hold.length > 0 ? hold : points;
	if (scored.length === 0) return 0;
	return (
		scored.reduce((sum, point) => sum + (Number.isFinite(point.rps) ? point.rps : 0), 0) /
		scored.length
	);
}

/** Mean core utilization for a bucket; `0` when the sampler recorded no cores (avoids NaN). */
export function meanCpu(point: TimeseriesPoint): number {
	if (!point.cpu || point.cpu.length === 0) return 0;
	const sum = point.cpu.reduce((total, value) => total + (Number.isFinite(value) ? value : 0), 0);
	return sum / point.cpu.length;
}
