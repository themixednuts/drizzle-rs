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

	const chartData = $derived.by(() => {
		if (points.length === 0) return { path: '', areaPath: '', yTicks: [] as { y: number; label: string }[], xTicks: [] as { x: number; label: string }[], dots: [] as { x: number; y: number; val: number; runId: string; git: string }[] };

		const vals = points.map((p) => p[metric]);
		const min = Math.min(...vals);
		const max = Math.max(...vals);
		const range = max - min || 1;

		const innerW = W - PAD_L - PAD_R;
		const innerH = H - PAD_T - PAD_B;
		const stepX = innerW / (vals.length - 1 || 1);

		const dots = vals.map((v, i) => {
			const x = PAD_L + i * stepX;
			const y = PAD_T + innerH - ((v - min) / range) * innerH;
			return { x, y, val: v, runId: points[i].run_id, git: points[i].git };
		});

		const path = dots.map((d, i) => (i === 0 ? 'M' : 'L') + d.x.toFixed(1) + ',' + d.y.toFixed(1)).join(' ');
		const lastDot = dots[dots.length - 1];
		const areaPath = path + ` L${lastDot.x.toFixed(1)},${PAD_T + innerH} L${PAD_L},${PAD_T + innerH} Z`;

		// Y-axis ticks (5 ticks)
		const yTicks = Array.from({ length: 5 }, (_, i) => {
			const val = min + (range * i) / 4;
			const y = PAD_T + innerH - (i / 4) * innerH;
			return { y, label: formatValue(val) };
		});

		// X-axis ticks (show ~8 labels max)
		const step = Math.max(1, Math.floor(points.length / 8));
		const xTicks = points
			.filter((_, i) => i % step === 0 || i === points.length - 1)
			.map((p, _, arr) => {
				const idx = points.indexOf(p);
				const x = PAD_L + idx * stepX;
				return { x, label: shortHash(p.git) };
			});

		return { path, areaPath, yTicks, xTicks, dots };
	});

	let hoveredIdx = $state<number | null>(null);
</script>

<div class="trend-chart">
	<div class="chart-label">{label}</div>
	<svg viewBox="0 0 {W} {H}" preserveAspectRatio="xMidYMid meet">
		<!-- Grid lines -->
		{#each chartData.yTicks as tick}
			<line x1={PAD_L} x2={W - PAD_R} y1={tick.y} y2={tick.y} stroke="var(--border)" stroke-width="0.5" />
			<text x={PAD_L - 8} y={tick.y + 3} fill="var(--text-muted)" font-size="9" text-anchor="end" font-family="var(--font-mono)">{tick.label}</text>
		{/each}

		<!-- X labels -->
		{#each chartData.xTicks as tick}
			<text x={tick.x} y={H - 8} fill="var(--text-muted)" font-size="8" text-anchor="middle" font-family="var(--font-mono)">{tick.label}</text>
		{/each}

		<!-- Area fill -->
		{#if chartData.areaPath}
			<defs>
				<linearGradient id="trend-grad-{metric}" x1="0" y1="0" x2="0" y2="1">
					<stop offset="0%" stop-color={color} stop-opacity="0.15" />
					<stop offset="100%" stop-color={color} stop-opacity="0" />
				</linearGradient>
			</defs>
			<path d={chartData.areaPath} fill="url(#trend-grad-{metric})" />
		{/if}

		<!-- Line -->
		{#if chartData.path}
			<path d={chartData.path} fill="none" stroke={color} stroke-width="2" stroke-linejoin="round" />
		{/if}

		<!-- Dots (interactive) -->
		{#each chartData.dots as dot, i}
			<circle
				cx={dot.x}
				cy={dot.y}
				r={hoveredIdx === i ? 5 : 2.5}
				fill={hoveredIdx === i ? color : 'var(--bg-surface)'}
				stroke={color}
				stroke-width="1.5"
				role="img"
				aria-label="{dot.runId}: {formatValue(dot.val)}"
				onmouseenter={() => hoveredIdx = i}
				onmouseleave={() => hoveredIdx = null}
			/>
		{/each}

		<!-- Tooltip -->
		{#if hoveredIdx !== null}
			{@const dot = chartData.dots[hoveredIdx]}
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
				>{formatValue(dot.val)}</text>
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
