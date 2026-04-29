<script lang="ts">
	import type { TimeseriesPoint } from '$lib/types';
	import { SparkLineState, SPARKLINE_VIEWBOX, type SparkLineMetric } from './spark-line.svelte';

	interface Props {
		points: TimeseriesPoint[];
		metric: SparkLineMetric;
	}

	let { points, metric }: Props = $props();
	const view = new SparkLineState(() => points, () => metric);
</script>

<div class="spark-wrap">
	<div class="spark-meta">
		<span>{view.metricLabel}</span>
		<span>latest {view.valueText} / {view.sampleText}</span>
	</div>
	<svg viewBox="0 0 {SPARKLINE_VIEWBOX.width} {SPARKLINE_VIEWBOX.height}" class="sparkline" preserveAspectRatio="none">
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
			<line
				x1={view.coordinates[0].x}
				x2={view.coordinates[0].x}
				y1="4"
				y2={SPARKLINE_VIEWBOX.height - 4}
				stroke={view.color}
				stroke-opacity="0.24"
				stroke-width="1"
			/>
			<circle
				cx={view.coordinates[0].x}
				cy={view.coordinates[0].y}
				r="4"
				fill={view.color}
			/>
		{/if}
	</svg>
</div>

<style>
	.spark-wrap {
		margin-top: 4px;
	}

	.spark-meta {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--ink-3);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.sparkline {
		width: 100%;
		height: 48px;
		display: block;
	}
</style>
