<script lang="ts">
	import type { TrendPoint } from '$lib/types';
	import {
		TrendChartState,
		TREND_CHART_VIEWBOX,
		type TrendChartMetric
	} from './trend-chart.svelte';

	interface Props {
		points: TrendPoint[];
		metric: TrendChartMetric;
		label: string;
		color: string;
		formatValue: (n: number) => string;
	}

	let { points, metric, label, color, formatValue }: Props = $props();
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
	<svg viewBox="0 0 {TREND_CHART_VIEWBOX.width} {TREND_CHART_VIEWBOX.height}" preserveAspectRatio="xMidYMid meet">
		<!-- Grid lines -->
		{#each view.chartData.yTicks as tick}
			<line x1={TREND_CHART_VIEWBOX.paddingLeft} x2={TREND_CHART_VIEWBOX.width - TREND_CHART_VIEWBOX.paddingRight} y1={tick.y} y2={tick.y} stroke="var(--border)" stroke-width="0.5" />
			<text x={TREND_CHART_VIEWBOX.paddingLeft - 8} y={tick.y + 3} fill="var(--text-muted)" font-size="9" text-anchor="end" font-family="var(--font-mono)">{tick.label}</text>
		{/each}

		<!-- X labels -->
		{#each view.chartData.xTicks as tick}
			<text x={tick.x} y={TREND_CHART_VIEWBOX.height - 8} fill="var(--text-muted)" font-size="8" text-anchor="middle" font-family="var(--font-mono)">{tick.label}</text>
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
