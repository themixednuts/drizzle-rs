<script lang="ts">
	import { Chart, getChartContext, Highlight, Rule, Spline, Svg } from 'layerchart';
	import ChartTip from './ChartTip.svelte';
	import { ssrBox } from '#lib/chart-ssr';
	import { cn } from '#lib/utils.js';
	import { fmtLatency, fmtRps } from '#lib/format';
	import type { OverlayChart, OverlaySeries } from '#lib/overlay';

	/**
	 * All of a run's targets on one chart, second by second.
	 *
	 * Height is fixed in CSS and passed to the server render as explicit pixels, so the plot occupies
	 * exactly the same box before and after hydration — this is the largest thing on the page and
	 * therefore the one most worth not reflowing. Everything interactive below is enhancement: with
	 * scripting off the lines and the legend still render, the legend is simply not clickable.
	 */
	let { chart, height = 190 }: { chart: OverlayChart; height?: number } = $props();

	interface Point {
		index: number;
		value: number | null;
	}

	/**
	 * Which series are hidden, by target id. Empty means "show everything", which is also what the
	 * server renders, so hydration never changes what is on screen.
	 */
	let hidden = $state<Record<string, boolean>>({});

	const visible = $derived(chart.series.filter((series) => !hidden[series.targetId]));
	const format = $derived(chart.metric === 'rps' ? fmtRps : fmtLatency);

	/** The x positions the chart is drawn over; series values are looked up by index. */
	const axis = $derived(Array.from({ length: chart.length }, (_, index) => ({ index })));

	function points(series: OverlaySeries): Point[] {
		return series.values.map((value, index) => ({ index, value }));
	}

	/** Drizzle takes the accent; the rest step down the neutral ramp so ours reads as figure. */
	function stroke(series: OverlaySeries): string {
		return series.isOurs ? 'var(--primary)' : `var(--series-${series.tone + 1})`;
	}

	/**
	 * Click to hide a line; click the last visible line to bring everything back. Soloing is what
	 * you actually want when four lines overlap, and this keeps one click meaningful in both
	 * directions rather than needing a modifier.
	 */
	function toggle(targetId: string): void {
		const others = chart.series.filter((series) => series.targetId !== targetId);
		const soloed = !hidden[targetId] && others.every((series) => hidden[series.targetId]);
		hidden = soloed ? {} : { ...hidden, [targetId]: !hidden[targetId] };
	}

	/** Which trial a bucket belongs to, for the tooltip's header line. */
	function trialAt(index: number): string {
		const trial = chart.trials.find((entry) => index >= entry.start && index < entry.end);
		if (!trial) return '';
		return trial.label.startsWith('trial') ? trial.label : `trial ${trial.label}`;
	}
</script>

{#snippet tip()}
	{@const ctx = getChartContext()}
	{@const datum = ctx.tooltip.data as Point | null}
	{#if datum}
		{@const index = datum.index}
		{@const trial = trialAt(index)}
		<ChartTip>
			<div class="text-micro text-muted-foreground font-mono uppercase">
				{trial}{trial ? ' · ' : ''}second {index + 1}
			</div>
			{#each visible as series (series.targetId)}
				{@const value = series.values[index]}
				<div class="flex items-center gap-2">
					<span class="h-[3px] w-3 shrink-0" style="background:{stroke(series)}" aria-hidden="true"
					></span>
					<span class="text-foreground-secondary flex-1 truncate">{series.name}</span>
					<span class="text-foreground font-mono tabular-nums">
						{value === null || value === undefined ? 'no sample' : format(value)}
					</span>
				</div>
			{/each}
		</ChartTip>
	{/if}
{/snippet}

<div>
	<div class="flex flex-wrap items-baseline gap-x-5 gap-y-2">
		<h2 class="text-lead font-semibold">{chart.title}</h2>
		{#if chart.series.length > 0}
			<!--
				Legend entries are buttons, not decoration: they toggle a line and report their own
				pressed state. They are also the >=24px hit target the bare swatch never was.
			-->
			<div class="ml-auto flex flex-wrap gap-x-1 gap-y-1" role="group" aria-label="series">
				{#each chart.series as series (series.targetId)}
					{@const off = hidden[series.targetId] === true}
					<button
						type="button"
						aria-pressed={!off}
						onclick={() => toggle(series.targetId)}
						title={off ? `Show ${series.name}` : `Hide ${series.name}`}
						class={cn(
							'text-meta hover:bg-muted flex min-h-6 items-center gap-1.5 px-1.5 py-1 transition-colors',
							off ? 'text-muted-foreground' : 'text-foreground-secondary',
						)}
					>
						<span
							class="h-[3px] w-3.5 shrink-0"
							style="background:{stroke(series)};opacity:{off ? 0.3 : 1}"
							aria-hidden="true"
						></span>
						<span class={cn(off && 'line-through')}>{series.name}</span>
					</button>
				{/each}
			</div>
		{/if}
	</div>

	{#if chart.unavailable}
		<div
			class="text-meta text-muted-foreground mt-4 flex items-center font-mono"
			style="height:{height}px"
		>
			{chart.unavailable}
		</div>
	{:else}
		<div class="mt-4 flex gap-3">
			<!-- Axis labels as text, not a drawn axis: three numbers is all this chart needs, and it
			     keeps the plot itself free of tick furniture. -->
			<div
				class="text-micro text-muted-foreground flex w-13 shrink-0 flex-col justify-between text-right font-mono"
				style="height:{height}px"
				aria-hidden="true"
			>
				<span>{chart.yTop}</span>
				<span>{chart.yMid}</span>
				<span>0</span>
			</div>

			<div class="bg-surface-inset min-w-0 flex-1 overflow-hidden" style="height:{height}px">
				<Chart
					data={axis}
					x="index"
					y={() => 0}
					xDomain={[0, Math.max(1, chart.length - 1)]}
					yDomain={[0, chart.max]}
					padding={{ top: 4, bottom: 4 }}
					tooltipContext={{ mode: 'bisect-x' }}
					{...ssrBox(720, height)}
				>
					<Svg>
						{#each [0.5, 1] as fraction (fraction)}
							<Rule y={chart.max * fraction} class="stroke-border" />
						{/each}
						{#each chart.breaks as position (position)}
							<Rule x={position} class="stroke-border [stroke-dasharray:3_4]" />
						{/each}
						{#each visible as series (series.targetId)}
							<Spline
								data={points(series)}
								x="index"
								y="value"
								defined={(d: Point) => d.value !== null}
								stroke={stroke(series)}
								strokeWidth={series.isOurs ? 1.9 : 1.2}
							/>
						{/each}
						<!-- Crosshair only. A dot per series would need one Highlight per line, and the
						     values are already spelled out in the tooltip. -->
						<Highlight lines />
					</Svg>

					{@render tip()}
				</Chart>
			</div>
		</div>

		{#if chart.trials.length > 1}
			<div
				class="text-micro text-muted-foreground ml-16 flex justify-between font-mono"
				aria-hidden="true"
			>
				{#each chart.trials as trial (trial.start)}
					<span>{trial.label}</span>
				{/each}
			</div>
		{/if}
	{/if}
</div>
