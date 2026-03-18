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

	function values(pts: TimeseriesPoint[], m: typeof metric): number[] {
		switch (m) {
			case 'rps': return pts.map((p) => p.rps);
			case 'latency': return pts.map((p) => p.latency.p95);
			case 'cpu': return pts.map((p) => p.cpu.reduce((a, b) => a + b, 0) / p.cpu.length);
			case 'mem': return pts.map((p) => p.mem_mb ?? 0);
		}
	}

	const path = $derived.by(() => {
		const vals = values(points, metric);
		if (vals.length === 0) return '';

		const min = Math.min(...vals);
		const max = Math.max(...vals);
		const range = max - min || 1;

		const stepX = (W - PAD * 2) / (vals.length - 1 || 1);

		return vals
			.map((v, i) => {
				const x = PAD + i * stepX;
				const y = H - PAD - ((v - min) / range) * (H - PAD * 2);
				return (i === 0 ? 'M' : 'L') + x.toFixed(1) + ',' + y.toFixed(1);
			})
			.join(' ');
	});

	const areaPath = $derived.by(() => {
		if (!path) return '';
		const vals = values(points, metric);
		const stepX = (W - PAD * 2) / (vals.length - 1 || 1);
		const lastX = PAD + (vals.length - 1) * stepX;
		return path + ` L${lastX.toFixed(1)},${H} L${PAD},${H} Z`;
	});

	const colorMap = { rps: 'var(--accent)', latency: 'var(--cyan)', cpu: 'var(--green)', mem: 'var(--purple, #a78bfa)' };
</script>

<svg viewBox="0 0 {W} {H}" class="sparkline" preserveAspectRatio="none">
	<defs>
		<linearGradient id="grad-{metric}" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0%" stop-color={colorMap[metric]} stop-opacity="0.2" />
			<stop offset="100%" stop-color={colorMap[metric]} stop-opacity="0" />
		</linearGradient>
	</defs>
	{#if areaPath}
		<path d={areaPath} fill="url(#grad-{metric})" />
	{/if}
	{#if path}
		<path d={path} fill="none" stroke={colorMap[metric]} stroke-width="1.5" />
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
