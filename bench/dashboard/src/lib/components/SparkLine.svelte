<script lang="ts">
	import { Area, Chart, Svg } from 'layerchart';
	import * as ChartUI from '#lib/components/ui/chart/index.js';
	import { metricChartConfig } from '#lib/metrics';
	import type { TimeseriesPoint } from '#lib/types';
	import { SparkLineState, type SparkLineMetric, type SparkPoint } from './spark-line.svelte';

	let {
		points,
		metric,
		trialCount = 1,
	}: {
		points: TimeseriesPoint[];
		metric: SparkLineMetric;
		trialCount?: number;
	} = $props();

	const view = new SparkLineState(() => ({ points, metric, trialCount }));
	const config = $derived(metricChartConfig(metric));
</script>

<div class="mt-1">
	<div
		class="text-micro text-muted-foreground flex justify-between gap-3 font-mono tracking-wide uppercase"
	>
		<span>{view.metricLabel}</span>
		<span>latest {view.valueText} / {view.sampleText}</span>
	</div>

	{#if view.hasSeries}
		<ChartUI.Container {config} class="aspect-auto h-12 w-full">
			<Chart
				data={view.series}
				x="index"
				y="value"
				yNice
				padding={{ top: 2, bottom: 2 }}
				pointerEvents={false}
			>
				<Svg>
					<Area
						y0={() => 0}
						defined={(d: SparkPoint) => d.value !== null}
						fill="var(--color-value)"
						fillOpacity={0.16}
						line={{ stroke: 'var(--color-value)', strokeWidth: 1.5 }}
					/>
				</Svg>
			</Chart>
		</ChartUI.Container>
	{:else}
		<div class="text-caption text-muted-foreground flex h-12 items-center font-mono">
			no samples in this trial
		</div>
	{/if}
</div>
