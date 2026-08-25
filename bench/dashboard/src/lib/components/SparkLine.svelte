<script lang="ts">
	import { Area, Chart, getChartContext, Highlight, Svg } from 'layerchart';
	import * as ChartUI from '#lib/components/ui/chart/index.js';
	import ChartTip from './ChartTip.svelte';
	import { METRICS, metricChartConfig } from '#lib/metrics';
	import { ssrBox } from '#lib/chart-ssr';
	import { toPoints, type SeriesPoint, type TargetChart } from '#lib/chart-series';

	/**
	 * The chart arrives already derived, from the server on first render and from the remote
	 * function on a client-side metric switch. This component only draws.
	 */
	let { chart }: { chart: TargetChart } = $props();

	const config = $derived(metricChartConfig(chart.metric));
	const format = $derived(METRICS[chart.metric].format);
</script>

<!-- A hover readout, so a sparkline can be read at a point instead of only as a shape. -->
{#snippet tip()}
	{@const ctx = getChartContext()}
	{@const datum = ctx.tooltip.data as SeriesPoint | null}
	{#if datum}
		<ChartTip>
			<div class="text-micro text-muted-foreground font-mono uppercase">
				second {datum.index + 1}
			</div>
			<div class="flex items-center gap-3">
				<span class="text-foreground-secondary">{chart.label}</span>
				<span class="text-foreground ml-auto font-mono tabular-nums">
					{datum.value === null ? 'no sample' : format(datum.value)}
				</span>
			</div>
		</ChartTip>
	{/if}
{/snippet}

<div class="mt-1">
	<div
		class="text-micro text-muted-foreground flex justify-between gap-3 font-mono tracking-wide uppercase"
	>
		<span>{chart.label}</span>
		<span>latest {chart.valueText} / {chart.sampleText}</span>
	</div>

	<!-- The 3rem box is reserved in the markup, so the chart, the empty state and the loading
	     placeholder all occupy identical space and nothing reflows when one replaces another. -->
	<div class="h-12 w-full overflow-hidden">
		{#if chart.hasSeries}
			<ChartUI.Container {config} class="aspect-auto h-12 w-full justify-start">
				<Chart
					data={toPoints(chart.series)}
					x="index"
					y="value"
					yNice
					padding={{ top: 2, bottom: 2 }}
					tooltipContext={{ mode: 'bisect-x' }}
					{...ssrBox(360, 48)}
				>
					<Svg>
						<Area
							y0={() => 0}
							defined={(d: SeriesPoint) => d.value !== null}
							fill="var(--color-value)"
							fillOpacity={0.16}
							line={{ stroke: 'var(--color-value)', strokeWidth: 1.5 }}
						/>
						<Highlight points lines />
					</Svg>

					{@render tip()}
				</Chart>
			</ChartUI.Container>
		{:else}
			<div class="text-label text-muted-foreground flex h-12 items-center font-mono">
				no samples in this trial
			</div>
		{/if}
	</div>
</div>
