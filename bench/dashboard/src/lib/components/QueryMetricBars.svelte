<script lang="ts">
	import { Area, Chart, Svg } from 'layerchart';
	import * as ChartUI from '#lib/components/ui/chart/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { METRICS, metricChartConfig } from '#lib/metrics';
	import { ssrBox } from '#lib/chart-ssr';
	import { toPoints, type SeriesPoint, type TargetChart } from '#lib/chart-series';

	/** Route-level breakdown of whichever metric the target's sparkline is showing. */
	let { chart }: { chart: TargetChart } = $props();

	const config = $derived(metricChartConfig(chart.metric));
	const format = $derived(METRICS[chart.metric].format);
	const heading = $derived(chart.metric === 'rps' ? 'route rps' : 'route p95');
</script>

<div class="mt-3">
	<Separator />
	<div
		class="text-micro text-muted-foreground mt-3 mb-2 flex justify-between gap-3 font-mono tracking-wide uppercase"
	>
		<span>{heading}</span>
		<span>{chart.sampleText}</span>
	</div>

	{#if !chart.routes || chart.routes.length === 0}
		<p class="measure border-border text-meta text-muted-foreground border-l-2 pl-3">
			{chart.routesUnavailable}
		</p>
	{:else}
		<ul class="grid gap-1.5">
			{#each chart.routes as route (route.id)}
				<li
					class="text-caption grid grid-cols-1 items-center gap-x-3 gap-y-1 font-mono tracking-normal sm:grid-cols-[minmax(8rem,1fr)_minmax(9rem,0.8fr)_9rem] {route.hasSamples
						? ''
						: 'opacity-45'}"
				>
					<div class="min-w-0">
						<div class="text-foreground truncate">{route.name}</div>
						<div class="text-muted-foreground truncate">{route.method} {route.path}</div>
					</div>
					<div class="text-muted-foreground flex flex-wrap gap-x-2.5 gap-y-1 tabular-nums">
						<span>avg {format(route.avg)}</span>
						<span>peak {format(route.peak)}</span>
						<span>latest {format(route.latest)}</span>
					</div>
					<!-- Height reserved so the row does not grow when the chart paints. -->
					<div class="h-7 w-full overflow-hidden">
						<ChartUI.Container {config} class="aspect-auto h-7 w-full justify-start">
							<Chart
								data={toPoints(route.series)}
								x="index"
								y="value"
								yDomain={[0, chart.routeMax]}
								padding={{ top: 2, bottom: 2 }}
								pointerEvents={false}
								{...ssrBox(150, 28)}
							>
								<Svg>
									<Area
										y0={() => 0}
										defined={(d: SeriesPoint) => d.value !== null}
										fill="var(--color-value)"
										fillOpacity={0.14}
										line={{ stroke: 'var(--color-value)', strokeWidth: 1.4 }}
									/>
								</Svg>
							</Chart>
						</ChartUI.Container>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</div>
