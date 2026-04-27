import type { TimeseriesPoint } from '$lib/types';

export type SparkLineMetric = 'rps' | 'latency' | 'cpu' | 'mem';

const W = 360;
const H = 60;
const PAD = 2;
const MID_Y = H / 2;

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

		return vals.map((value, index) => {
			const x = vals.length === 1 ? W / 2 : PAD + index * stepX;
			const y = range === 0 ? MID_Y : H - PAD - ((value - min) / range) * (H - PAD * 2);
			return { x, y };
		});
	});

	path = $derived.by(() => {
		if (this.coordinates.length < 2) return '';

		return this.coordinates
			.map(({ x, y }, index) => (index === 0 ? 'M' : 'L') + x.toFixed(1) + ',' + y.toFixed(1))
			.join(' ');
	});

	areaPath = $derived.by(() => {
		if (!this.path) return '';
		const lastX = this.coordinates[this.coordinates.length - 1].x;
		return this.path + ` L${lastX.toFixed(1)},${H} L${PAD},${H} Z`;
	});

	colorMap = {
		rps: 'var(--accent)',
		latency: 'var(--cyan)',
		cpu: 'var(--green)',
		mem: 'var(--purple, #a78bfa)'
	};
	color = $derived(this.colorMap[this.metric]);

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
