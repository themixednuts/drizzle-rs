import { shortHash } from '$lib/format';
import type { TrendPoint } from '$lib/types';

export type TrendChartMetric =
	| 'rps_avg'
	| 'rps_peak'
	| 'latency_p95'
	| 'latency_p99'
	| 'cpu_avg'
	| 'err';

const W = 800;
const H = 200;
const PAD_T = 24;
const PAD_B = 32;
const PAD_L = 60;
const PAD_R = 16;

type ChartDot = { x: number; y: number; val: number; runId: string; git: string };
type ChartData = {
	path: string;
	areaPath: string;
	yTicks: { y: number; label: string }[];
	xTicks: { x: number; label: string }[];
	dots: ChartDot[];
};

const emptyChartData = (): ChartData => ({
	path: '',
	areaPath: '',
	yTicks: [],
	xTicks: [],
	dots: []
});

export class TrendChartState {
	#points: () => TrendPoint[];
	#metric: () => TrendChartMetric;
	#label: () => string;
	#color: () => string;
	#formatValue: () => (value: number) => string;
	hoveredIdx = $state<number | null>(null);

	chartData = $derived.by(() => {
		if (this.points.length === 0) return emptyChartData();

		const vals = this.points.map((point) => point[this.metric]);
		const min = Math.min(...vals);
		const max = Math.max(...vals);
		const range = max - min || 1;

		const innerW = W - PAD_L - PAD_R;
		const innerH = H - PAD_T - PAD_B;
		const stepX = innerW / (vals.length - 1 || 1);

		const dots = vals.map((value, index) => {
			const x = PAD_L + index * stepX;
			const y = PAD_T + innerH - ((value - min) / range) * innerH;
			return {
				x,
				y,
				val: value,
				runId: this.points[index].run_id,
				git: this.points[index].git
			};
		});

		const path = dots
			.map((dot, index) => (index === 0 ? 'M' : 'L') + dot.x.toFixed(1) + ',' + dot.y.toFixed(1))
			.join(' ');
		const lastDot = dots[dots.length - 1];
		const areaPath =
			path + ` L${lastDot.x.toFixed(1)},${PAD_T + innerH} L${PAD_L},${PAD_T + innerH} Z`;

		const yTicks = Array.from({ length: 5 }, (_, index) => {
			const value = min + (range * index) / 4;
			const y = PAD_T + innerH - (index / 4) * innerH;
			return { y, label: this.formatValue(value) };
		});

		const step = Math.max(1, Math.floor(this.points.length / 8));
		const xTicks = this.points
			.map((point, index) => ({ point, index }))
			.filter(({ index }) => index % step === 0 || index === this.points.length - 1)
			.map(({ point, index }) => ({
				x: PAD_L + index * stepX,
				label: shortHash(point.git)
			}));

		return { path, areaPath, yTicks, xTicks, dots };
	});

	hoveredDot = $derived(
		this.hoveredIdx === null ? null : (this.chartData.dots[this.hoveredIdx] ?? null)
	);

	constructor(
		points: () => TrendPoint[],
		metric: () => TrendChartMetric,
		label: () => string,
		color: () => string,
		formatValue: () => (value: number) => string
	) {
		this.#points = points;
		this.#metric = metric;
		this.#label = label;
		this.#color = color;
		this.#formatValue = formatValue;
	}

	get points() {
		return this.#points();
	}

	get metric() {
		return this.#metric();
	}

	get label() {
		return this.#label();
	}

	get color() {
		return this.#color();
	}

	get formatValue() {
		return this.#formatValue();
	}

	hover = (index: number): void => {
		this.hoveredIdx = index;
	};

	clearHover = (): void => {
		this.hoveredIdx = null;
	};
}

export const TREND_CHART_VIEWBOX = {
	width: W,
	height: H,
	paddingLeft: PAD_L,
	paddingRight: PAD_R
};
