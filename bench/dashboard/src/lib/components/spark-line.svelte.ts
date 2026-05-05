import type { TimeseriesPoint } from '$lib/types';
import { fmtCpu, fmtLatency, fmtRps } from '$lib/format';

export type SparkLineMetric = 'rps' | 'latency' | 'cpu' | 'mem';

const W = 360;
const H = 60;
const PAD = 2;
const MID_Y = H / 2;
const MIN_GAP_MS = 10_000;

interface Coordinate {
	x: number;
	y: number;
	breakBefore: boolean;
}

function values(points: TimeseriesPoint[], metric: SparkLineMetric): number[] {
	switch (metric) {
		case 'rps':
			return points.map((point) => point.rps);
		case 'latency':
			return points.map((point) => point.latency.p95);
		case 'cpu':
			return points.map((point) => point.cpu.reduce((sum, value) => sum + value, 0) / point.cpu.length);
		case 'mem':
			return points.map((point) => point.mem_mb ?? 0);
	}
}

export class SparkLineState {
	#points: () => TimeseriesPoint[];
	#metric: () => SparkLineMetric;
	coordinates = $derived.by(() => {
		const vals = values(this.points, this.metric);
		if (vals.length === 0) return [];

		const min = Math.min(...vals);
		const max = Math.max(...vals);
		const range = max - min;
		const stepX = vals.length > 1 ? (W - PAD * 2) / (vals.length - 1) : 0;
		const gaps = trialGaps(this.points);

		return vals.map((value, index) => {
			const x = vals.length === 1 ? W / 2 : PAD + index * stepX;
			const y = range === 0 ? MID_Y : H - PAD - ((value - min) / range) * (H - PAD * 2);
			return { x, y, breakBefore: gaps.has(index) };
		});
	});

	path = $derived.by(() => {
		if (this.coordinates.length < 2) return '';

		return this.coordinates
			.map(
				({ x, y, breakBefore }, index) =>
					(index === 0 || breakBefore ? 'M' : 'L') + x.toFixed(1) + ',' + y.toFixed(1)
			)
			.join(' ');
	});

	areaPath = $derived.by(() => {
		return areaSegments(this.coordinates);
	});

	colorMap = {
		rps: 'var(--accent)',
		latency: 'var(--ink-2)',
		cpu: 'var(--ink-2)',
		mem: 'var(--ink-2)'
	};
	color = $derived(this.colorMap[this.metric]);

	latestValue = $derived.by(() => {
		const vals = values(this.points, this.metric);
		return vals.at(-1) ?? null;
	});

	valueText = $derived.by(() => {
		const value = this.latestValue;
		if (value === null) return 'no samples';
		switch (this.metric) {
			case 'rps':
				return fmtRps(value);
			case 'latency':
				return fmtLatency(value);
			case 'cpu':
				return fmtCpu(value);
			case 'mem':
				return `${value.toFixed(1)}MB`;
		}
	});

	metricLabel = $derived.by(() => {
		if (this.metric === 'latency') return 'p95';
		return this.metric;
	});

	sampleText = $derived(`${this.points.length} bucket${this.points.length === 1 ? '' : 's'}`);

	constructor(points: () => TimeseriesPoint[], metric: () => SparkLineMetric) {
		this.#points = points;
		this.#metric = metric;
	}

	get points() {
		return this.#points();
	}

	get metric() {
		return this.#metric();
	}
}

export const SPARKLINE_VIEWBOX = { width: W, height: H };

function trialGaps(points: TimeseriesPoint[]): Set<number> {
	const times = points.map((point) => Date.parse(point.time));
	const deltas = times
		.slice(1)
		.map((time, index) => time - times[index])
		.filter((delta) => Number.isFinite(delta) && delta > 0)
		.sort((a, b) => a - b);
	const medianDelta = deltas[Math.floor(deltas.length / 2)] ?? 0;
	const threshold = Math.max(MIN_GAP_MS, medianDelta * 5);
	const gaps = new Set<number>();

	for (let index = 1; index < times.length; index += 1) {
		const delta = times[index] - times[index - 1];
		if (Number.isFinite(delta) && delta > threshold) {
			gaps.add(index);
		}
	}

	return gaps;
}

function areaSegments(coordinates: Coordinate[]): string {
	const segments: Coordinate[][] = [];
	let segment: Coordinate[] = [];

	for (const coordinate of coordinates) {
		if (coordinate.breakBefore && segment.length > 0) {
			segments.push(segment);
			segment = [];
		}
		segment.push(coordinate);
	}
	if (segment.length > 0) segments.push(segment);

	return segments
		.filter((item) => item.length > 1)
		.map((item) => {
			const first = item[0];
			const last = item[item.length - 1];
			const path = item
				.map(({ x, y }, index) => (index === 0 ? 'M' : 'L') + x.toFixed(1) + ',' + y.toFixed(1))
				.join(' ');
			return `${path} L${last.x.toFixed(1)},${H} L${first.x.toFixed(1)},${H} Z`;
		})
		.join(' ');
}
