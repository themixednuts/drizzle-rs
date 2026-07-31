import { fmtCpu, fmtLatency, fmtPct, fmtRps } from './format';
import type { ChartConfig } from './components/ui/chart/index.js';

/**
 * The metrics this dashboard draws, and the one place each one's name, hue and formatter is
 * decided.
 *
 * Every chart builds its shadcn `ChartConfig` from here, so throughput is the same amber on a
 * sparkline, a trend line and a route breakdown, and latency is the same blue everywhere it
 * appears. The hues are the validated categorical palette in `app.css`.
 */
export type MetricKey = 'rps' | 'latency' | 'cpu' | 'mem' | 'err';

export interface MetricDefinition {
	key: MetricKey;
	/** Short label used on chart headers and tab triggers. */
	label: string;
	/** CSS custom property holding this metric's hue. */
	color: string;
	format: (value: number) => string;
}

export const METRICS: Record<MetricKey, MetricDefinition> = {
	rps: { key: 'rps', label: 'rps', color: 'var(--metric-rps)', format: fmtRps },
	latency: { key: 'latency', label: 'p95', color: 'var(--metric-latency)', format: fmtLatency },
	cpu: { key: 'cpu', label: 'cpu', color: 'var(--metric-cpu)', format: fmtCpu },
	mem: {
		key: 'mem',
		label: 'mem',
		color: 'var(--metric-mem)',
		format: (value) => `${value.toFixed(1)}MB`,
	},
	err: { key: 'err', label: 'err', color: 'var(--metric-err)', format: fmtPct },
};

/**
 * A single-series chart config keyed by `value`, so every chart in the app reads its colour as
 * `var(--color-value)` regardless of which metric it is showing.
 */
export function metricChartConfig(metric: MetricKey, label?: string): ChartConfig {
	const definition = METRICS[metric];
	return { value: { label: label ?? definition.label, color: definition.color } };
}
