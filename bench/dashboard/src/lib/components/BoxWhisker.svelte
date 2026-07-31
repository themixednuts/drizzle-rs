<script lang="ts">
	import { BoxPlot, Chart, getChartContext, Rule, Svg } from 'layerchart';
	import { scaleBand } from 'd3-scale';
	import ChartTip from './ChartTip.svelte';
	import { cn } from '#lib/utils.js';
	import { ssrBox } from '#lib/chart-ssr';
	import type { BoxWhiskerDatum, BoxWhiskerExtent } from '#lib/boxplot';

	/**
	 * Trial spread for one target row.
	 *
	 * Quartiles are drawn only when the artifact recorded them. When it recorded a min/max range
	 * but no quartiles, LayerChart's box collapses onto the median because q1/q3 are pinned to the
	 * range ends — the summary line under the chart is what states which of the three cases this
	 * is, so a reader is never shown an invented box.
	 */
	let {
		box,
		extent,
		label,
		summaryLabel = '',
		accent = false,
	}: {
		box: BoxWhiskerDatum;
		extent: BoxWhiskerExtent;
		/** Full spoken description, used as the chart's accessible name. */
		label: string;
		summaryLabel?: string;
		/** True on the drizzle baseline row. */
		accent?: boolean;
	} = $props();

	const hasQuartiles = $derived(box.q1 !== null && box.q3 !== null);
	const median = $derived(box.median ?? box.min);

	const datum = $derived({
		category: 'spread',
		min: box.min,
		q1: hasQuartiles ? (box.q1 as number) : median,
		median,
		q3: hasQuartiles ? (box.q3 as number) : median,
		max: box.max,
	});
</script>

<!--
	The spread, spelled out. This used to be a `title=` attribute, which never appears on touch,
	is not reachable by keyboard, and looks nothing like the tooltip every other chart shows.
-->
{#snippet tip()}
	{@const ctx = getChartContext()}
	{#if ctx.tooltip.data}
		<ChartTip class="max-w-72">
			<span class="text-pretty">{label}</span>
		</ChartTip>
	{/if}
{/snippet}

<div class="grid min-w-0 gap-1.5">
	<div class="border-border bg-muted/50 h-6 w-full border" role="img" aria-label={label}>
		<Chart
			data={[datum]}
			x="min"
			y="category"
			yScale={scaleBand().padding(0.12)}
			xDomain={[extent.min, extent.min + extent.span]}
			valueAxis="x"
			padding={{ left: 1, right: 1 }}
			tooltipContext={{ mode: 'band' }}
			{...ssrBox(210, 24)}
		>
			<Svg>
				{#if box.spread !== 'none'}
					<BoxPlot
						data={datum}
						min="min"
						q1="q1"
						median="median"
						q3="q3"
						max="max"
						capWidth={0.9}
						class={cn(
							'stroke-foreground-secondary',
							accent ? '[&.lc-boxplot-box]:fill-primary' : '[&.lc-boxplot-box]:fill-primary/45',
						)}
					/>
				{:else}
					<!-- No per-trial spread was recorded: a single tick, not a fabricated bar. -->
					<Rule x={median} class="stroke-foreground stroke-2" />
				{/if}
			</Svg>

			{@render tip()}
		</Chart>
	</div>
	<!--
		The min/median/max caption used to sit under every whisker, which put a second line of numbers
		in a column whose entire job is to show a shape. It is the same text the tooltip already
		carries, so on a wide screen only the glyph shows; the caption returns below `lg`, where the
		whisker is small enough that the numbers are doing the work instead of the drawing.
	-->
	{#if summaryLabel}
		<span class="text-micro text-muted-foreground truncate font-mono lg:hidden">
			{summaryLabel}
		</span>
	{/if}
</div>
