import type { BoxMetric, MinMax, Summary, VarianceMetric } from './types';

export const compareCategoryOptions = [
	{ value: 'rps', label: 'RPS' },
	{ value: 'latency', label: 'Latency' },
	{ value: 'cpu', label: 'CPU' },
	{ value: 'mem', label: 'Memory' },
	{ value: 'err', label: 'Errors' }
] as const;

export const compareMetricOptions = [
	{ value: 'rps.avg', label: 'RPS (avg)' },
	{ value: 'rps.peak', label: 'RPS (peak)' },
	{ value: 'latency.avg', label: 'Latency (avg)' },
	{ value: 'latency.p90', label: 'Latency (p90)' },
	{ value: 'latency.p95', label: 'Latency (p95)' },
	{ value: 'latency.p99', label: 'Latency (p99)' },
	{ value: 'latency.p999', label: 'Latency (p999)' },
	{ value: 'cpu.avg', label: 'CPU (avg)' },
	{ value: 'cpu.peak', label: 'CPU (peak)' },
	{ value: 'mem.avg', label: 'Memory (avg)' },
	{ value: 'mem.peak', label: 'Memory (peak)' },
	{ value: 'err', label: 'Error rate' }
] as const;

export interface CompareCategoryColumn {
	key: string;
	label: string;
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

export interface CompareBoxPlot {
	label: string;
	min: number;
	q1: number;
	median: number;
	q3: number;
	max: number;
	samples: number;
}

export type CompareCategory = (typeof compareCategoryOptions)[number]['value'];
export type CompareMetric = (typeof compareMetricOptions)[number]['value'];

export const defaultCompareCategory: CompareCategory = 'rps';
export const defaultCompareMetric: CompareMetric = 'rps.avg';

export const compareCategoryColumns: Record<CompareCategory, CompareCategoryColumn[]> = {
	rps: [
		{ key: 'avg', label: 'avg' },
		{ key: 'peak', label: 'peak' }
	],
	latency: [
		{ key: 'avg', label: 'avg' },
		{ key: 'p90', label: 'p90' },
		{ key: 'p95', label: 'p95' },
		{ key: 'p99', label: 'p99' },
		{ key: 'p999', label: 'p999' }
	],
	cpu: [
		{ key: 'avg', label: 'avg' },
		{ key: 'peak', label: 'peak' }
	],
	mem: [
		{ key: 'avg', label: 'avg' },
		{ key: 'peak', label: 'peak' }
	],
	err: [{ key: 'rate', label: 'rate' }]
};

const compareCategoryValues = new Set<string>(compareCategoryOptions.map((category) => category.value));
const compareMetricValues = new Set<string>(compareMetricOptions.map((metric) => metric.value));

export function isCompareCategory(
	category: string | null | undefined
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

export function parseCompareMetric(metric: string | null | undefined): CompareMetric {
	return isCompareMetric(metric) ? metric : defaultCompareMetric;
}

export function compareCategoryLabel(category: CompareCategory): string {
	return compareCategoryOptions.find((option) => option.value === category)?.label ?? category;
}

export function isHigherBetterCategory(category: CompareCategory): boolean {
	return category === 'rps';
}

export function isHigherBetterMetric(metric: CompareMetric): boolean {
	return metric.startsWith('rps');
}

export function extractCompareCategoryValues(
	summary: Summary,
	category: CompareCategory
): CompareCategoryValue[] | null {
	const primary = summary.primary;

	switch (category) {
		case 'rps':
			return [
				{ key: 'avg', label: 'avg', value: primary.rps.avg },
				{ key: 'peak', label: 'peak', value: primary.rps.peak }
			];
		case 'latency':
			return [
				{ key: 'avg', label: 'avg', value: primary.latency.avg },
				{ key: 'p90', label: 'p90', value: primary.latency.p90 },
				{ key: 'p95', label: 'p95', value: primary.latency.p95 },
				{ key: 'p99', label: 'p99', value: primary.latency.p99 },
				{ key: 'p999', label: 'p999', value: primary.latency.p999 }
			];
		case 'cpu':
			return [
				{ key: 'avg', label: 'avg', value: primary.cpu.avg },
				{ key: 'peak', label: 'peak', value: primary.cpu.peak }
			];
		case 'mem':
			return primary.mem
				? [
						{ key: 'avg', label: 'avg', value: primary.mem.avg },
						{ key: 'peak', label: 'peak', value: primary.mem.peak }
					]
				: null;
		case 'err':
			return [{ key: 'rate', label: 'rate', value: primary.err }];
	}
}

export function extractCompareCategorySortValue(
	summary: Summary,
	category: CompareCategory
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
	category: CompareCategory
): CompareVariance | null {
	const primary = summary.primary;
	const trials = summary.spread.trials;

	switch (category) {
		case 'rps':
			return {
				label: 'rps sample variance',
				...varianceOrZero(summary.spread.variance?.rps, trials)
			};
		case 'latency':
			return {
				label: 'p95 sample variance',
				...varianceOrZero(summary.spread.variance?.p95, trials)
			};
		case 'cpu':
			return {
				label: 'cpu avg sample variance',
				...varianceOrZero(summary.spread.variance?.cpu, trials)
			};
		case 'mem':
			return primary.mem
				? {
						label: 'memory avg sample variance',
						...varianceOrZero(summary.spread.variance?.mem, trials)
					}
				: null;
		case 'err':
			return {
				label: 'error-rate sample variance',
				...varianceOrZero(summary.spread.variance?.err, trials)
			};
	}
}

export function extractCompareCategoryBox(summary: Summary, category: CompareCategory): CompareBoxPlot | null {
	const primary = summary.primary;
	const trials = summary.spread.trials;
	const boxplot = summary.spread.boxplot;

	switch (category) {
		case 'rps':
			return boxOrRange('rps across trials', boxplot?.rps, summary.spread.rps, primary.rps.avg, trials);
		case 'latency':
			return boxOrRange('p95 across trials', boxplot?.p95, summary.spread.p95, primary.latency.p95, trials);
		case 'cpu':
			return boxOrStdev('cpu avg across trials', boxplot?.cpu, primary.cpu.avg, summary.spread.variance?.cpu, trials);
		case 'mem':
			return primary.mem
				? boxOrStdev(
						'memory avg across trials',
						boxplot?.mem,
						primary.mem.avg,
						summary.spread.variance?.mem,
						trials
					)
				: null;
		case 'err':
			return boxOrStdev('error rate across trials', boxplot?.err, primary.err, summary.spread.variance?.err, trials);
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
		case 'latency.p90':
			return primary.latency.p90;
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

function boxOrRange(
	label: string,
	box: BoxMetric | undefined,
	range: MinMax,
	median: number,
	trials: number
): CompareBoxPlot {
	if (box) return { label, ...box };
	return {
		label,
		min: range.min,
		q1: range.min,
		median,
		q3: range.max,
		max: range.max,
		samples: trials
	};
}

function boxOrStdev(
	label: string,
	box: BoxMetric | undefined,
	median: number,
	variance: VarianceMetric | undefined,
	trials: number
): CompareBoxPlot {
	if (box) return { label, ...box };
	const stdev = variance?.stdev ?? 0;
	return {
		label,
		min: Math.max(0, median - stdev),
		q1: Math.max(0, median - stdev / 2),
		median,
		q3: median + stdev / 2,
		max: median + stdev,
		samples: variance?.samples ?? trials
	};
}
