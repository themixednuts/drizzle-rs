<script lang="ts">
	import { Area, Chart, Svg } from 'layerchart';
	import * as ChartUI from '#lib/components/ui/chart/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { metricChartConfig } from '#lib/metrics';
	import type { QueryDoc, TimeseriesPoint } from '#lib/types';
	import { QueryMetricBarsState } from './query-metric-bars.svelte';
	import type { SparkLineMetric, SparkPoint } from './spark-line.svelte';

	let {
		queries,
		points,
		metric,
		trialCount = 1,
	}: {
		queries: QueryDoc[];
		points: TimeseriesPoint[];
		metric: SparkLineMetric;
		trialCount?: number;
	} = $props();

	const view = new QueryMetricBarsState(() => ({ queries, points, metric, trialCount }));
	const config = $derived(metricChartConfig(metric));
</script>

<div class="mt-3">
	<Separator />
	<div
		class="text-micro text-muted-foreground mt-3 mb-2 flex justify-between gap-3 font-mono tracking-wide uppercase"
	>
		<span>{view.metricLabel}</span>
		<span>{view.sampleText}</span>
	</div>

	{#if !view.isAttributable || !view.hasQueryMetrics}
		<p class="measure border-border text-meta text-muted-foreground border-l-2 pl-3">
			{view.unavailableText}
		</p>
	{:else}
		<ul class="grid gap-1.5">
			{#each view.rows as row (row.query.id)}
				<li
					class="text-caption grid grid-cols-1 items-center gap-x-3 gap-y-1 font-mono tracking-normal sm:grid-cols-[minmax(8rem,1fr)_minmax(9rem,0.8fr)_9rem] {row.hasSamples
						? ''
						: 'opacity-45'}"
				>
					<div class="min-w-0">
						<div class="text-foreground truncate">{row.query.name}</div>
						<div class="text-muted-foreground truncate">{row.query.method} {row.query.path}</div>
					</div>
					<div class="text-muted-foreground flex flex-wrap gap-x-2.5 gap-y-1 tabular-nums">
						<span>avg {view.format(row.avg)}</span>
						<span>peak {view.format(row.peak)}</span>
						<span>latest {view.format(row.latest)}</span>
					</div>
					<ChartUI.Container {config} class="aspect-auto h-7 w-full">
						<Chart
							data={row.series}
							x="index"
							y="value"
							yDomain={[0, view.maxValue]}
							padding={{ top: 2, bottom: 2 }}
							pointerEvents={false}
						>
							<Svg>
								<Area
									y0={() => 0}
									defined={(d: SparkPoint) => d.value !== null}
									fill="var(--color-value)"
									fillOpacity={0.14}
									line={{ stroke: 'var(--color-value)', strokeWidth: 1.4 }}
								/>
							</Svg>
						</Chart>
					</ChartUI.Container>
				</li>
			{/each}
		</ul>
	{/if}
</div>
