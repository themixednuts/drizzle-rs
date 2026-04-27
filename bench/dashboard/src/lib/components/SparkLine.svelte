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
