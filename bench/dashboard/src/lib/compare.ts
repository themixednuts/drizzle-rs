import { boxDatum, pointDatum, rangeDatum } from './boxplot';
import type { BoxMetric, MinMax, SpreadDatum, Summary, VarianceMetric } from './types';

export const compareCategoryOptions = [
	{ value: 'rps', label: 'RPS' },
	{ value: 'latency', label: 'Latency' },
	{ value: 'cpu', label: 'CPU' },
	{ value: 'mem', label: 'Memory' },
	{ value: 'err', label: 'Errors' },
] as const;

export const compareMetricOptions = [
	{ value: 'rps.avg', label: 'RPS (median across trials)' },
	{ value: 'rps.peak', label: 'RPS (peak bucket)' },
	{ value: 'latency.avg', label: 'Latency (mean)' },
	{ value: 'latency.p50', label: 'Latency (p50)' },
	{ value: 'latency.p90', label: 'Latency (p90)' },
	{ value: 'latency.p95', label: 'Latency (p95)' },
	{ value: 'latency.p99', label: 'Latency (p99)' },
	{ value: 'latency.p999', label: 'Latency (p999)' },
	{ value: 'cpu.avg', label: 'CPU (median across trials)' },
	{ value: 'cpu.peak', label: 'CPU (peak core)' },
	{ value: 'mem.avg', label: 'Memory (median across trials)' },
	{ value: 'mem.peak', label: 'Memory (peak)' },
	{ value: 'err', label: 'Error rate' },
] as const;

export interface CompareCategoryColumn {
	key: string;
	label: string;
	/** Rendered as the column `title`, so the aggregation is never implied by the label alone. */
	hint: string;
}

export interface CompareCategoryValue extends CompareCategoryColumn {
	value: number;
}

export interface CompareVariance {
	label: string;
	value: number;
	stdev: number;
	samples: number;
}

export interface CompareBoxPlot extends SpreadDatum {
	label: string;
}

export type CompareCategory = (typeof compareCategoryOptions)[number]['value'];
export type CompareMetric = (typeof compareMetricOptions)[number]['value'];

export const defaultCompareCategory: CompareCategory = 'rps';
export const defaultCompareMetric: CompareMetric = 'rps.avg';

const CROSS_TRIAL_MEDIAN = 'median across trials (summary aggregate)';

/**
 * Column definitions per category.
 *
 * The runner's `*.avg` summary keys hold the *median across trials*, so every column that
 * renders one is labelled `median`, not `avg`. `latency.avg` is the one exception: it is the
 * median across trials of each trial's mean latency, which is why its hint spells that out.
 *
 * `latency.p50`/`latency.p90` only appear when the artifact actually measured them — see
 * `latencyColumns`.
 */
export const compareCategoryColumns: Record<CompareCategory, CompareCategoryColumn[]> = {
	rps: [
		{ key: 'avg', label: 'median', hint: `requests/second, ${CROSS_TRIAL_MEDIAN}` },
		{ key: 'peak', label: 'peak', hint: 'highest single sample bucket across trials' },
	],
	latency: [
		{ key: 'avg', label: 'mean', hint: `mean latency per trial, ${CROSS_TRIAL_MEDIAN}` },
		{
			key: 'p50',
			label: 'p50',
			hint: `50th percentile of measured samples, ${CROSS_TRIAL_MEDIAN}`,
		},
		{
			key: 'p90',
			label: 'p90',
			hint: `90th percentile of measured samples, ${CROSS_TRIAL_MEDIAN}`,
		},
		{
			key: 'p95',
			label: 'p95',
			hint: `95th percentile of measured samples, ${CROSS_TRIAL_MEDIAN}`,
		},
		{
			key: 'p99',
			label: 'p99',
			hint: `99th percentile of measured samples, ${CROSS_TRIAL_MEDIAN}`,
		},
		{
			key: 'p999',
			label: 'p999',
			hint: `99.9th percentile of measured samples, ${CROSS_TRIAL_MEDIAN}`,
		},
	],
	cpu: [
		{ key: 'avg', label: 'median', hint: `mean-across-cores utilization, ${CROSS_TRIAL_MEDIAN}` },
		{ key: 'peak', label: 'peak core', hint: 'highest single-core utilization observed' },
	],
	mem: [
		{ key: 'avg', label: 'median', hint: `resident MB, ${CROSS_TRIAL_MEDIAN}` },
		{ key: 'peak', label: 'peak', hint: 'highest resident MB observed' },
	],
	err: [{ key: 'rate', label: 'rate', hint: 'errored requests / total requests' }],
};

const compareCategoryValues = new Set<string>(
	compareCategoryOptions.map((category) => category.value),
);
const compareMetricValues = new Set<string>(compareMetricOptions.map((metric) => metric.value));

export function isCompareCategory(
	category: string | null | undefined,
): category is CompareCategory {
	return typeof category === 'string' && compareCategoryValues.has(category);
}

export function isCompareMetric(metric: string | null | undefined): metric is CompareMetric {
	return typeof metric === 'string' && compareMetricValues.has(metric);
}

export function parseCompareCategory(category: string | null | undefined): CompareCategory {
	if (isCompareCategory(category)) return category;
	if (isCompareMetric(category)) return metricCategory(category);
	return defaultCompareCategory;
}

export function compareCategoryLabel(category: CompareCategory): string {
	return compareCategoryOptions.find((option) => option.value === category)?.label ?? category;
}

export function isHigherBetterCategory(category: CompareCategory): boolean {
	return category === 'rps';
}

/**
 * Older summaries carried an interpolated `p90` and no `p50` at all. A present `p50` is the
 * marker that the runner measured real percentiles, so `p50`/`p90` columns are only offered
 * when at least one row has one.
 */
export function hasMeasuredPercentiles(summary: Summary): boolean {
	return summary.primary.latency.p50 != null;
}

/** Drop `p50`/`p90` columns when no row in the set has measured values for them. */
export function visibleCategoryColumns(
	category: CompareCategory,
	summaries: readonly { values: { key: string; value: number }[] }[],
): CompareCategoryColumn[] {
	const columns = compareCategoryColumns[category];
	if (category !== 'latency') return columns;

	const present = new Set(summaries.flatMap((item) => item.values.map((value) => value.key)));
	return columns.filter((column) => {
		if (column.key !== 'p50' && column.key !== 'p90') return true;
		return present.has(column.key);
	});
}

export function extractCompareCategoryValues(
	summary: Summary,
	category: CompareCategory,
): CompareCategoryValue[] | null {
	const primary = summary.primary;
	const columns = compareCategoryColumns[category];
	const column = (key: string) => columns.find((item) => item.key === key)!;
	const cell = (key: string, value: number): CompareCategoryValue => ({ ...column(key), value });

	switch (category) {
		case 'rps':
			return [cell('avg', primary.rps.avg), cell('peak', primary.rps.peak)];
		case 'latency': {
			const values = [cell('avg', primary.latency.avg)];
			// Only measured percentiles are surfaced; an absent p50 marks a legacy artifact
			// whose p90 was interpolated, so that column is dropped rather than shown.
			if (primary.latency.p50 != null) values.push(cell('p50', primary.latency.p50));
			if (primary.latency.p50 != null && primary.latency.p90 != null) {
				values.push(cell('p90', primary.latency.p90));
			}
			values.push(
				cell('p95', primary.latency.p95),
				cell('p99', primary.latency.p99),
				cell('p999', primary.latency.p999),
			);
			return values;
		}
		case 'cpu':
			return [cell('avg', primary.cpu.avg), cell('peak', primary.cpu.peak)];
		case 'mem':
			return primary.mem ? [cell('avg', primary.mem.avg), cell('peak', primary.mem.peak)] : null;
		case 'err':
			return [cell('rate', primary.err)];
	}
}

export function extractCompareCategorySortValue(
	summary: Summary,
	category: CompareCategory,
): number | null {
	switch (category) {
		case 'rps':
			return summary.primary.rps.avg;
		case 'latency':
			return summary.primary.latency.p95;
		case 'cpu':
			return summary.primary.cpu.avg;
		case 'mem':
			return summary.primary.mem?.avg ?? null;
		case 'err':
			return summary.primary.err;
	}
}

export function extractCompareCategoryVariance(
	summary: Summary,
	category: CompareCategory,
): CompareVariance | null {
	const primary = summary.primary;
	const trials = summary.spread.trials;

	switch (category) {
		case 'rps':
			return {
				label: 'rps sample variance',
				...varianceOrZero(summary.spread.variance?.rps, trials),
			};
		case 'latency':
			return {
				label: 'p95 sample variance',
				...varianceOrZero(summary.spread.variance?.p95, trials),
			};
		case 'cpu':
			return {
				label: 'cpu sample variance',
				...varianceOrZero(summary.spread.variance?.cpu, trials),
			};
		case 'mem':
			return primary.mem
				? {
						label: 'memory sample variance',
						...varianceOrZero(summary.spread.variance?.mem, trials),
					}
				: null;
		case 'err':
			return {
				label: 'error-rate sample variance',
				...varianceOrZero(summary.spread.variance?.err, trials),
			};
	}
}

export function extractCompareCategoryBox(
	summary: Summary,
	category: CompareCategory,
): CompareBoxPlot | null {
	const primary = summary.primary;
	const trials = summary.spread.trials;
	const boxplot = summary.spread.boxplot;

	switch (category) {
		case 'rps':
			return spreadOrRange(
				'rps across trials',
				boxplot?.rps,
				summary.spread.rps,
				primary.rps.avg,
				trials,
			);
		case 'latency':
			return spreadOrRange(
				'p95 across trials',
				boxplot?.p95,
				summary.spread.p95,
				primary.latency.p95,
				trials,
			);
		case 'cpu':
			return spreadOrPoint('cpu across trials', boxplot?.cpu, primary.cpu.avg, trials);
		case 'mem':
			return primary.mem
				? spreadOrPoint('memory across trials', boxplot?.mem, primary.mem.avg, trials)
				: null;
		case 'err':
			return spreadOrPoint('error rate across trials', boxplot?.err, primary.err, trials);
	}
}

export function extractCompareMetric(summary: Summary, metric: CompareMetric): number | null {
	const primary = summary.primary;

	switch (metric) {
		case 'rps.avg':
			return primary.rps.avg;
		case 'rps.peak':
			return primary.rps.peak;
		case 'latency.avg':
			return primary.latency.avg;
		case 'latency.p50':
			return primary.latency.p50 ?? null;
		case 'latency.p90':
			// Legacy artifacts interpolated p90; a present p50 marks a measured artifact.
			return primary.latency.p50 != null ? (primary.latency.p90 ?? null) : null;
		case 'latency.p95':
			return primary.latency.p95;
		case 'latency.p99':
			return primary.latency.p99;
		case 'latency.p999':
			return primary.latency.p999;
		case 'cpu.avg':
			return primary.cpu.avg;
		case 'cpu.peak':
			return primary.cpu.peak;
		case 'mem.avg':
			return primary.mem?.avg ?? null;
		case 'mem.peak':
			return primary.mem?.peak ?? null;
		case 'err':
			return primary.err;
	}
}

function metricCategory(metric: CompareMetric): CompareCategory {
	if (metric.startsWith('rps')) return 'rps';
	if (metric.startsWith('latency')) return 'latency';
	if (metric.startsWith('cpu')) return 'cpu';
	if (metric.startsWith('mem')) return 'mem';
	return 'err';
}

function varianceOrZero(metric: VarianceMetric | undefined, trials: number): VarianceMetric {
	return metric ?? { value: 0, stdev: 0, samples: trials };
}

/** Real boxplot when present, otherwise the recorded min/max range with no invented quartiles. */
function spreadOrRange(
	label: string,
	box: BoxMetric | undefined,
	range: MinMax,
	median: number,
	trials: number,
): CompareBoxPlot {
	if (box) return { label, ...boxDatum(box) };
	return {
		label: `${label} (min/max range, no quartiles recorded)`,
		...rangeDatum(range, median, trials),
	};
}

/**
 * Metrics with no recorded per-trial min/max. Previously a stdev was expanded into fake
 * quartiles; now the aggregate is rendered as a single tick and labelled as such.
 */
function spreadOrPoint(
	label: string,
	box: BoxMetric | undefined,
	median: number,
	trials: number,
): CompareBoxPlot {
	if (box) return { label, ...boxDatum(box) };
	return { label: `${label} (no per-trial spread recorded)`, ...pointDatum(median, trials) };
}
