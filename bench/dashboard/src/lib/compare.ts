import type { Summary } from './types';

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

export type CompareMetric = (typeof compareMetricOptions)[number]['value'];

export const defaultCompareMetric: CompareMetric = 'rps.avg';

const compareMetricValues = new Set<string>(compareMetricOptions.map((metric) => metric.value));

export function isCompareMetric(metric: string | null | undefined): metric is CompareMetric {
	return typeof metric === 'string' && compareMetricValues.has(metric);
}

export function parseCompareMetric(metric: string | null | undefined): CompareMetric {
	return isCompareMetric(metric) ? metric : defaultCompareMetric;
}

export function isHigherBetterMetric(metric: CompareMetric): boolean {
	return metric.startsWith('rps');
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
