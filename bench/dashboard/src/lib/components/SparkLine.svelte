<script lang="ts">
	import type { TimeseriesPoint } from '$lib/types';

	interface Props {
		points: TimeseriesPoint[];
		metric: 'rps' | 'latency' | 'cpu' | 'mem';
	}

	let { points, metric }: Props = $props();

	const W = 360;
	const H = 60;
	const PAD = 2;
	const MID_Y = H / 2;

	function values(pts: TimeseriesPoint[], m: Props['metric']): number[] {
		switch (m) {
			case 'rps': return pts.map((p) => p.rps);
			case 'latency': return pts.map((p) => p.latency.p95);
			case 'cpu': return pts.map((p) => p.cpu.reduce((a, b) => a + b, 0) / p.cpu.length);
			case 'mem': return pts.map((p) => p.mem_mb ?? 0);
		}
	}

	class SparkLineState {
		#points: () => TimeseriesPoint[];
		#metric: () => Props['metric'];
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

		constructor(points: () => TimeseriesPoint[], metric: () => Props['metric']) {
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

	const view = new SparkLineState(() => points, () => metric);
</script>

<svg viewBox="0 0 {W} {H}" class="sparkline" preserveAspectRatio="none">
	<defs>
		<linearGradient id="grad-{view.metric}" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0%" stop-color={view.color} stop-opacity="0.2" />
			<stop offset="100%" stop-color={view.color} stop-opacity="0" />
		</linearGradient>
	</defs>
	{#if view.areaPath}
		<path d={view.areaPath} fill="url(#grad-{view.metric})" />
	{/if}
	{#if view.path}
		<path d={view.path} fill="none" stroke={view.color} stroke-width="1.5" />
	{/if}
	{#if view.coordinates.length === 1}
		<circle
			cx={view.coordinates[0].x}
			cy={view.coordinates[0].y}
			r="2.5"
			fill={view.color}
		/>
	{/if}
</svg>

<style>
	.sparkline {
		width: 100%;
		height: 48px;
		display: block;
		margin-top: 4px;
	}
</style>
