<script lang="ts">
	import type { TrendPoint } from '$lib/types';
	import { shortHash } from '$lib/format';

	interface Props {
		points: TrendPoint[];
		metric: 'rps_avg' | 'rps_peak' | 'latency_p95' | 'latency_p99' | 'cpu_avg' | 'err';
		label: string;
		color: string;
		formatValue: (n: number) => string;
	}

	let { points, metric, label, color, formatValue }: Props = $props();

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

	class TrendChartState {
		#points: () => TrendPoint[];
		#metric: () => Props['metric'];
		#label: () => string;
		#color: () => string;
		#formatValue: () => Props['formatValue'];
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
			metric: () => Props['metric'],
			label: () => string,
			color: () => string,
			formatValue: () => Props['formatValue']
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

	const view = new TrendChartState(
		() => points,
		() => metric,
		() => label,
		() => color,
		() => formatValue
	);
</script>

<div class="trend-chart">
	<div class="chart-label">{view.label}</div>
	<svg viewBox="0 0 {W} {H}" preserveAspectRatio="xMidYMid meet">
		<!-- Grid lines -->
		{#each view.chartData.yTicks as tick}
			<line x1={PAD_L} x2={W - PAD_R} y1={tick.y} y2={tick.y} stroke="var(--border)" stroke-width="0.5" />
			<text x={PAD_L - 8} y={tick.y + 3} fill="var(--text-muted)" font-size="9" text-anchor="end" font-family="var(--font-mono)">{tick.label}</text>
		{/each}

		<!-- X labels -->
		{#each view.chartData.xTicks as tick}
			<text x={tick.x} y={H - 8} fill="var(--text-muted)" font-size="8" text-anchor="middle" font-family="var(--font-mono)">{tick.label}</text>
		{/each}

		<!-- Area fill -->
		{#if view.chartData.areaPath}
			<defs>
				<linearGradient id="trend-grad-{view.metric}" x1="0" y1="0" x2="0" y2="1">
					<stop offset="0%" stop-color={view.color} stop-opacity="0.15" />
					<stop offset="100%" stop-color={view.color} stop-opacity="0" />
				</linearGradient>
			</defs>
			<path d={view.chartData.areaPath} fill="url(#trend-grad-{view.metric})" />
		{/if}

		<!-- Line -->
		{#if view.chartData.path}
			<path d={view.chartData.path} fill="none" stroke={view.color} stroke-width="2" stroke-linejoin="round" />
		{/if}

		<!-- Dots (interactive) -->
		{#each view.chartData.dots as dot, i}
			<circle
				cx={dot.x}
				cy={dot.y}
				r={view.hoveredIdx === i ? 5 : 2.5}
				fill={view.hoveredIdx === i ? view.color : 'var(--bg-surface)'}
				stroke={view.color}
				stroke-width="1.5"
				role="img"
				aria-label="{dot.runId}: {view.formatValue(dot.val)}"
				onmouseenter={() => view.hover(i)}
				onmouseleave={view.clearHover}
			/>
		{/each}

		<!-- Tooltip -->
		{#if view.hoveredDot}
			{@const dot = view.hoveredDot}
			<g>
				<rect
					x={dot.x - 56}
					y={dot.y - 46}
					width="112"
					height="36"
					rx="4"
					fill="var(--bg-raised)"
					stroke="var(--border)"
					stroke-width="1"
				/>
				<text
					x={dot.x}
					y={dot.y - 31}
					fill="var(--text-primary)"
					font-size="11"
					text-anchor="middle"
					font-family="var(--font-mono)"
					font-weight="600"
				>{view.formatValue(dot.val)}</text>
				<text
					x={dot.x}
					y={dot.y - 18}
					fill="var(--text-muted)"
					font-size="8"
					text-anchor="middle"
					font-family="var(--font-mono)"
				>{dot.git.slice(0, 7)}</text>
			</g>
		{/if}
	</svg>
</div>

<style>
	.trend-chart {
		margin-bottom: 24px;
	}

	.chart-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
		margin-bottom: 8px;
	}

	svg {
		width: 100%;
		height: auto;
		display: block;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
	}

	circle {
		cursor: pointer;
		transition: r 0.15s;
	}
</style>
