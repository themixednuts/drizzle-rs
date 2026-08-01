import type { BoxMetric, MinMax, SpreadDatum, Summary } from './types';

export interface BoxWhiskerExtent {
	min: number;
	span: number;
}

export type BoxWhiskerDatum = SpreadDatum;

/** A real boxplot recorded by the runner. */
export function boxDatum(box: BoxMetric): BoxWhiskerDatum {
	return {
		spread: 'boxplot',
		min: box.min,
		max: box.max,
		median: box.median,
		q1: box.q1,
		q3: box.q3,
		samples: box.samples,
	};
}

/**
 * Honest fallback when the artifact recorded a min/max range but no quartiles.
 * Quartiles stay null instead of being invented from the range or a stdev.
 */
export function rangeDatum(range: MinMax, median: number | null, samples: number): BoxWhiskerDatum {
	return {
		spread: 'range',
		min: range.min,
		max: range.max,
		median,
		q1: null,
		q3: null,
		samples,
	};
}

/** No per-trial spread recorded at all — render a single tick, not a fabricated bar. */
export function pointDatum(value: number, samples: number): BoxWhiskerDatum {
	return { spread: 'none', min: value, max: value, median: value, q1: null, q3: null, samples };
}

export function rpsBox(summary: Summary): BoxWhiskerDatum {
	const box = summary.spread.boxplot?.rps;
	if (box) return boxDatum(box);
	return rangeDatum(summary.spread.rps, summary.primary.rps.avg, summary.spread.trials);
}

export function boxWhiskerExtent(
	boxes: readonly BoxWhiskerDatum[],
	extraValues: readonly number[] = [],
): BoxWhiskerExtent {
	const values: number[] = [];
	for (const box of boxes) {
		values.push(box.min, box.max);
		if (box.median !== null) values.push(box.median);
		if (box.q1 !== null) values.push(box.q1);
		if (box.q3 !== null) values.push(box.q3);
	}
	values.push(...extraValues);

	const finite = values.filter((value) => Number.isFinite(value));
	if (finite.length === 0) return { min: 0, span: 1 };

	const min = Math.min(...finite);
	const max = Math.max(...finite);
	if (min === max) return { min: Math.max(0, min - 1), span: 2 };

	const span = max - min;
	const pad = span * 0.04;
	return { min: Math.max(0, min - pad), span: span + pad * 2 };
}
