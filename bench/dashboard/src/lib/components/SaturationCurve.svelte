<script lang="ts">
	import { Axis, Chart, getChartContext, Highlight, Point, Rule, Spline, Svg } from 'layerchart';
	import ChartTip from './ChartTip.svelte';
	import Hint from './Hint.svelte';
	import DataTable from './data/DataTable.svelte';
	import Td from './data/Td.svelte';
	import Th from './data/Th.svelte';
	import Tr from './data/Tr.svelte';
	import * as Table from '#lib/components/ui/table/index.js';
	import { ssrBox } from '#lib/chart-ssr';
	import { cn } from '#lib/utils.js';
	import { fmtCpu, fmtLatency, fmtPct, fmtRps } from '#lib/format';
	import type { CurvePoint, CurveView } from '#lib/saturation';

	/**
	 * Throughput vs concurrency, and latency vs concurrency, for one target.
	 *
	 * This is the most intuitive thing the saturation suite produces: a headline number is a claim,
	 * but the shape of the ramp is the evidence for it. Throughput climbing and then flattening while
	 * latency turns up is what "peak" means, drawn.
	 *
	 * Three deliberate choices:
	 *
	 * - **The x axis is ordinal, not linear.** A concurrency ramp is geometric (8, 16, 32, 64, ...),
	 *   so a linear axis crushes every early step into the first inch and the knee lands off in the
	 *   corner. Even spacing is the reading the data is about — one step per step.
	 * - **The line runs through every step, including disqualified ones.** A step that blew the error
	 *   budget is real data with a reason attached; hiding it would leave a gap in the curve that
	 *   nothing explains. It is drawn struck through, and it can never be the peak.
	 * - **The step table below is not a fallback for the chart, it is the audit.** Every number the
	 *   chart draws is in it, in text, without scripting — including each disqualification reason,
	 *   which a tooltip alone would hide from anyone who cannot hover.
	 */
	let {
		curve,
		targetName,
	}: {
		curve: CurveView;
		/** Used in the accessible names, so a page of several curves is navigable. */
		targetName: string;
	} = $props();

	const RPS_HEIGHT = 176;
	const LATENCY_HEIGHT = 148;
	/** Identical on both plots so the two share one x geometry and the peak rule lines up. */
	const PADDING = { top: 10, right: 14, bottom: 4, left: 58 };
	const X_PADDING = { ...PADDING, bottom: 24 };

	const points = $derived(curve.points);
	const ticks = $derived(points.map((point) => point.index));
	const lastIndex = $derived(Math.max(1, points.length - 1));
	const peakConcurrency = $derived(
		curve.peakIndex === null ? null : points[curve.peakIndex].concurrency,
	);

	function concurrencyAt(index: number): string {
		return String(points[index]?.concurrency ?? '');
	}

	/** Marker geometry per verdict. Shape carries the meaning; colour only reinforces it. */
	function markerClass(point: CurvePoint): string {
		if (point.disqualified !== null) return 'fill-background stroke-negative';
		if (point.isPeak) return 'fill-primary stroke-primary';
		if (point.sloMet) return 'fill-metric-rps stroke-metric-rps';
		return 'fill-background stroke-muted-foreground';
	}

	const VERDICT_TONE: Record<CurvePoint['verdict'], string> = {
		peak: 'text-signal',
		under: 'text-foreground-secondary',
		over: 'text-muted-foreground',
		disqualified: 'text-negative',
	};
</script>

{#snippet marker(point: CurvePoint)}
	<Point d={point}>
		{#snippet children({ x, y }: { x: number; y: number })}
			<g class={markerClass(point)} stroke-width="1.6">
				<circle cx={x} cy={y} r={point.isPeak ? 5.5 : 4} />
				{#if point.disqualified !== null}
					<!-- Struck through: the step was measured, and it is out of the running. -->
					<line x1={x - 3.2} y1={y + 3.2} x2={x + 3.2} y2={y - 3.2} class="stroke-negative" />
				{/if}
				{#if point.isPeak}
					<circle cx={x} cy={y} r="9" class="stroke-primary fill-none" stroke-width="1.2" />
				{/if}
			</g>
		{/snippet}
	</Point>
{/snippet}

{#snippet tip(showThroughput: boolean)}
	{@const ctx = getChartContext()}
	{@const datum = ctx.tooltip.data as CurvePoint | null}
	{#if datum}
		<ChartTip>
			<div class="text-micro text-muted-foreground font-mono uppercase">
				{datum.concurrency} concurrent requests
			</div>
			{#if showThroughput}
				<div class="flex items-center gap-3">
					<span class="text-foreground-secondary flex-1">throughput</span>
					<span class="text-foreground font-mono tabular-nums">{fmtRps(datum.rps)}</span>
				</div>
			{/if}
			{#each [['p50', datum.p50], ['p95', datum.p95], ['p99', datum.p99]] as [label, value] (label)}
				<div class="flex items-center gap-3">
					<span class="text-foreground-secondary flex-1">{label}</span>
					<span class="text-foreground font-mono tabular-nums">{fmtLatency(value as number)}</span>
				</div>
			{/each}
			<div class="flex items-center gap-3">
				<span class="text-foreground-secondary flex-1">errors / cpu</span>
				<span class="text-foreground font-mono tabular-nums">
					{fmtPct(datum.err)} / {fmtCpu(datum.cpu)}
				</span>
			</div>
			<div class={cn('border-border-soft mt-0.5 border-t pt-1', VERDICT_TONE[datum.verdict])}>
				{datum.verdictText}
			</div>
		</ChartTip>
	{/if}
{/snippet}

<figure class="mt-6">
	<figcaption class="flex flex-wrap items-baseline gap-x-4 gap-y-1">
		<h3 class="text-lead font-semibold">Throughput vs concurrency</h3>
		<span class="text-meta text-muted-foreground">
			<Hint
				hint="Unpaced closed loop: with no think time, N virtual users are N in-flight requests, so each step of the ramp is a fixed concurrency held long enough to measure steady state."
			>
				unpaced ramp, one point per concurrency step
			</Hint>
		</span>
	</figcaption>

	{#if points.length === 0}
		<div
			class="border-border text-label text-muted-foreground mt-3 flex items-center justify-center border border-dashed font-mono"
			style="height:{RPS_HEIGHT + LATENCY_HEIGHT}px"
		>
			this run recorded an outcome but published no curve
		</div>
	{:else}
		<div class="border-border mt-3 border">
			<!-- Throughput. Heights are fixed in CSS and handed to the server render as pixels, so the
			     box is identical before and after hydration. -->
			<div style="height:{RPS_HEIGHT}px">
				<Chart
					data={points}
					x="index"
					y="rps"
					xDomain={[0, lastIndex]}
					yDomain={[0, curve.rpsMax]}
					padding={PADDING}
					tooltipContext={{ mode: 'bisect-x' }}
					{...ssrBox(720, RPS_HEIGHT)}
				>
					<Svg>
						<Axis
							placement="left"
							grid
							ticks={4}
							format={(value: number) => fmtRps(value)}
							aria-label="requests per second for {targetName}"
						/>
						{#if curve.peakIndex !== null}
							<Rule x={curve.peakIndex} class="stroke-primary [stroke-dasharray:3_3]" />
						{/if}
						<Spline stroke="var(--metric-rps)" strokeWidth={1.8} />
						{#each points as point (point.index)}
							{@render marker(point)}
						{/each}
						<Highlight lines />
					</Svg>
					{@render tip(true)}
				</Chart>
			</div>

			<!-- Latency, sharing the x geometry above. The objective is a rule on this plot because
			     that is the axis it is stated on. -->
			<div class="border-border-soft border-t" style="height:{LATENCY_HEIGHT}px">
				<Chart
					data={points}
					x="index"
					y="p99"
					xDomain={[0, lastIndex]}
					yDomain={[0, curve.latencyMax]}
					padding={X_PADDING}
					tooltipContext={{ mode: 'bisect-x' }}
					{...ssrBox(720, LATENCY_HEIGHT)}
				>
					<Svg>
						<Axis
							placement="left"
							grid
							ticks={3}
							format={(value: number) => (value === 0 ? '0' : fmtLatency(value))}
							aria-label="latency for {targetName}"
						/>
						<Axis
							placement="bottom"
							{ticks}
							format={(value: number) => concurrencyAt(value)}
							aria-label="concurrency for {targetName}"
						/>
						<Rule y={curve.slo.ms} class="stroke-negative [stroke-dasharray:4_3]" />
						{#if curve.peakIndex !== null}
							<Rule x={curve.peakIndex} class="stroke-primary [stroke-dasharray:3_3]" />
						{/if}
						<Spline y="p95" stroke="var(--series-3)" strokeWidth={1.2} />
						<Spline stroke="var(--metric-latency)" strokeWidth={1.8} />
						<Highlight lines />
					</Svg>
					{@render tip(false)}
				</Chart>
			</div>
		</div>

		<div class="text-micro text-muted-foreground mt-2 text-center font-mono uppercase">
			concurrent requests
		</div>

		<!-- The legend names every mark on the plot, including the two that carry a judgement. -->
		<ul class="text-meta text-muted-foreground mt-3 flex flex-wrap gap-x-5 gap-y-1.5">
			<li class="flex items-center gap-1.5">
				<span class="bg-metric-rps h-[3px] w-3.5" aria-hidden="true"></span> throughput
			</li>
			<li class="flex items-center gap-1.5">
				<span class="bg-metric-latency h-[3px] w-3.5" aria-hidden="true"></span> p99
			</li>
			<li class="flex items-center gap-1.5">
				<span class="bg-series-3 h-[3px] w-3.5" aria-hidden="true"></span> p95
			</li>
			<li class="flex items-center gap-1.5">
				<span class="border-negative h-[3px] w-3.5 border-t border-dashed" aria-hidden="true"
				></span>
				objective {curve.sloLabel}
			</li>
			{#if curve.peakIndex !== null}
				<li class="flex items-center gap-1.5">
					<span class="bg-primary size-2 rounded-full" aria-hidden="true"></span>
					<Hint
						hint="The fastest step whose steady state stayed within the objective and stayed inside the error limit. Ties go to the lower concurrency, since the same throughput for less concurrency is the better result."
					>
						peak
					</Hint>
				</li>
			{/if}
			{#if curve.disqualifiedCount > 0}
				<li class="text-negative flex items-center gap-1.5">
					<span class="border-negative size-2 rounded-full border bg-transparent" aria-hidden="true"
					></span>
					<Hint
						hint="A step that exceeded the run's error limit. It is measured and drawn, and it is disqualified from being the peak. The reason is on each step in the table below."
					>
						{curve.disqualifiedCount} disqualified step{curve.disqualifiedCount === 1 ? '' : 's'}
					</Hint>
				</li>
			{/if}
		</ul>

		{#if curve.tallerThanPeak}
			<!--
				The artifact disagrees with its own curve. The peak is meant to be the fastest qualifying
				step, so this should never fire — except on runs selected under the earlier "highest
				qualifying concurrency" rule, which are still readable here and which name a peak several
				percent below the tallest qualifying step on any ramp that dips once the pool saturates.
				Same class of disclosure as `peakMissing`: state it rather than let the mark silently
				contradict the line a reader is looking at.
			-->
			<p class="measure text-meta text-muted-foreground mt-3">
				This run named its peak at {peakConcurrency} concurrent, but {curve.tallerThanPeak
					.concurrency} concurrent reached
				<span class="font-mono">{fmtRps(curve.tallerThanPeak.rps)}</span> and stayed within the objective
				too. The headline is the figure the run published; the peak is meant to be the fastest qualifying
				step, and on this artifact it is not.
			</p>
		{/if}

		{#if curve.peakMissing}
			<p class="text-meta text-negative mt-3">
				This artifact names a peak concurrency that is not present in its own curve, so no step is
				marked. The headline number is reported as published; the marker is withheld rather than
				attached to the nearest step.
			</p>
		{/if}

		<details class="border-border-soft group mt-4 border-t pt-3">
			<summary
				class="text-body text-muted-foreground hover:text-foreground flex cursor-pointer list-none items-center gap-1.5 marker:content-['']"
			>
				<span aria-hidden="true" class="inline-block transition-transform group-open:rotate-90">
					&rsaquo;
				</span>
				every step, and why each one is or is not the peak
			</summary>
			<div class="mt-3">
				<DataTable minWidth="min-w-[44rem]">
					<Table.Header>
						<Tr>
							<Th numeric>concurrency</Th>
							<Th numeric>rps</Th>
							<Th numeric>p95</Th>
							<Th numeric>p99</Th>
							<Th numeric>errors</Th>
							<Th numeric>cpu</Th>
							<Th>verdict</Th>
						</Tr>
					</Table.Header>
					<Table.Body>
						{#each points as point (point.index)}
							<Tr>
								<Td numeric>{point.concurrency}</Td>
								<Td numeric>{fmtRps(point.rps)}</Td>
								<Td numeric>{fmtLatency(point.p95)}</Td>
								<Td numeric>{fmtLatency(point.p99)}</Td>
								<Td numeric>{fmtPct(point.err)}</Td>
								<Td numeric>{fmtCpu(point.cpu)}</Td>
								<Td wrap class={VERDICT_TONE[point.verdict]}>{point.verdictText}</Td>
							</Tr>
						{/each}
					</Table.Body>
				</DataTable>
			</div>
		</details>
	{/if}
</figure>
