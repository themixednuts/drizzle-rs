<script lang="ts">
	import { Axis, Bars, Chart, getChartContext, Highlight, Labels, Svg } from 'layerchart';
	import { scaleBand } from 'd3-scale';
	import * as ChartUI from '#lib/components/ui/chart/index.js';
	import ChartTip from './ChartTip.svelte';
	import { metricChartConfig } from '#lib/metrics';
	import { fmtLatency } from '#lib/format';
	import { ssrBox } from '#lib/chart-ssr';
	import type { LatencyPercentiles } from '#lib/types';
	import { LatencyBarsState, type LatencyTier } from './latency-bars.svelte';

	let { latency }: { latency: LatencyPercentiles } = $props();

	const view = new LatencyBarsState(() => ({ latency }));
	const config = metricChartConfig('latency', 'latency distribution');
	// 24px per row rather than 18: a bar is a hover target now, and a 24px row is the smallest one
	// a pointer can hit reliably.
	const height = $derived(view.tiers.length * 24 + 8);
</script>

<!-- Each tier already carries the sentence that explains it; hovering is how you read it, instead
     of a legend that would have to repeat all six. -->
{#snippet tip()}
	{@const ctx = getChartContext()}
	{@const datum = ctx.tooltip.data as LatencyTier | null}
	{#if datum}
		<ChartTip class="max-w-64">
			<div class="flex items-center gap-3">
				<span class="text-foreground-secondary font-mono uppercase">{datum.label}</span>
				<span class="text-foreground ml-auto font-mono tabular-nums">
					{fmtLatency(datum.value)}
				</span>
			</div>
			<div class="text-muted-foreground text-pretty">{datum.hint}</div>
		</ChartTip>
	{/if}
{/snippet}

<ChartUI.Container {config} class="aspect-auto w-full" style="height: {height}px">
	<Chart
		data={view.tiers}
		x="value"
		y="label"
		yScale={scaleBand().padding(0.3)}
		xDomain={[0, view.maxValue]}
		padding={{ left: 34, right: 56 }}
		tooltipContext={{ mode: 'band' }}
		{...ssrBox(420, height)}
	>
		<Svg>
			<Axis placement="left" />
			<!-- The tail percentiles carry the metric hue; the body of the distribution stays neutral,
			     so the eye lands on p99/p999 without the chart shouting. Two layers rather than a
			     per-bar fill function, because a bar's fill is one colour by design. -->
			<Bars data={view.body} radius={0} fill="var(--color-foreground-faint)" />
			<Bars data={view.tail} radius={0} fill="var(--color-value)" />
			<Highlight area />
			<Labels
				placement="outside"
				class="fill-foreground-secondary font-mono"
				format={(value: number) => fmtLatency(value)}
			/>
		</Svg>

		{@render tip()}
	</Chart>
</ChartUI.Container>
