import type { BoxMetric, MinMax, Summary } from './types';

export interface BoxWhiskerExtent {
	min: number;
	span: number;
}

export type BoxWhiskerDatum = BoxMetric;

export function rpsBox(summary: Summary): BoxWhiskerDatum {
	return (
		summary.spread.boxplot?.rps ?? {
			min: summary.spread.rps.min,
			q1: summary.spread.rps.min,
			median: summary.primary.rps.avg,
			q3: summary.spread.rps.max,
			max: summary.spread.rps.max,
			samples: summary.spread.trials
		}
	);
}

export function rangeBox(range: MinMax, median: number, samples: number): BoxWhiskerDatum {
	return {
		min: range.min,
		q1: range.min,
		median,
		q3: range.max,
		max: range.max,
		samples
	};
}

export function boxWhiskerExtent(
	boxes: readonly BoxWhiskerDatum[],
	extraValues: readonly number[] = []
): BoxWhiskerExtent {
	const values = boxes.flatMap((box) => [box.min, box.q1, box.median, box.q3, box.max]);
	values.push(...extraValues);
	const min = Math.min(...values);
	const max = Math.max(...values);

	if (!Number.isFinite(min) || !Number.isFinite(max)) return { min: 0, span: 1 };
	if (min === max) return { min: Math.max(0, min - 1), span: 2 };

	const span = max - min;
	const pad = span * 0.04;
	return { min: Math.max(0, min - pad), span: span + pad * 2 };
}
