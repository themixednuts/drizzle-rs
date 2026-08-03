<script lang="ts">
	import ApiTag from './ApiTag.svelte';
	import CapacityFigure from './CapacityFigure.svelte';
	import Delta from './Delta.svelte';
	import Hint from './Hint.svelte';
	import OsBadge from './OsBadge.svelte';
	import { cn } from '#lib/utils.js';
	import { fmtCpu, fmtLatency, fmtPct, fmtRps, runStamp, shardLabel } from '#lib/format';
	import type { HarnessRow } from '#lib/harness';
	import type { QualitativeNote } from '#lib/qualitative';
	import type { RankingRow, RankingSort } from '#lib/ranking';
	import type { TargetDisplay } from '#lib/target-display';

	/**
	 * One row of the ranking.
	 *
	 * Closed, it is seven columns: rank, library (with its API tag and note), database, machine,
	 * a bar, throughput, p95. Opened, it is every other number the old sectioned table used to show
	 * inline — mean and p99 latency, cpu, errors, memory, the across-trial spread, the delta against
	 * drizzle-rs on this row's own database, and a link to the shard it came from.
	 *
	 * Three of those columns exist because the table is one global list rather than a stack of
	 * per-database bands: `database` is the family context the bands used to supply by position,
	 * the machine badge is the confound that matters most when adjacent rows came from different
	 * CI VMs, and the API tag is what tells two rows both named "Drizzle RS" apart.
	 *
	 * A native `<details>` rather than a JS disclosure: it is keyboard operable, announces its own
	 * expanded state, and works with scripting off — which is the only way "quieter, but still
	 * reachable" can be true for a reader who never gets the bundle.
	 */
	let {
		row,
		display,
		db,
		dbDetail,
		spread,
		spreadDetail,
		sort,
		variant = null,
		harness = null,
		showCapacity = false,
	}: {
		row: RankingRow;
		display: TargetDisplay;
		db: string;
		dbDetail: string;
		spread: string;
		spreadDetail: string;
		sort: RankingSort;
		/** Short form plus full text for the target's SQL notes; `null` when it declared none. */
		variant?: QualitativeNote | null;
		/** The harness this row's whole database ran under; `null` when nothing was declared. */
		harness?: HarnessRow | null;
		/** Whether this set measured capacity at all — see `RunsPageState.hasCapacity`. */
		showCapacity?: boolean;
	} = $props();

	const p = $derived(row.summary.primary);
	/**
	 * A dash, not a number, when the sorted column did not measure this row. Padding a null to "00"
	 * would put a rank on a row that has no place in this order.
	 */
	const rank = $derived(row.rank === null ? '—' : String(row.rank).padStart(2, '0'));
	const shard = $derived(`shard ${runStamp(row.summary.run_id)}`);
</script>

<details
	class={cn(
		'group border-border-soft border-b transition-colors last:border-b-0',
		// Identity: this row is drizzle-rs. Deliberately faint — see `--accent-tint` in app.css.
		row.isOurs && 'bg-accent-tint',
		// Attention: the row under the pointer, or holding keyboard focus, always wins. `hover:` and
		// `focus-within:` carry a pseudo-class, so they out-specify the identity tint above on the
		// drizzle row too.
		'hover:bg-accent-tint-strong focus-within:bg-accent-tint-strong',
	)}
>
	<summary
		class={cn(
			"grid cursor-pointer list-none grid-cols-[1.75rem_minmax(0,1fr)_auto] items-center gap-x-3 px-5 py-4 transition-colors marker:content-[''] lg:gap-x-5 lg:px-6",
			showCapacity
				? 'lg:grid-cols-[2rem_minmax(8rem,1fr)_5.5rem_3.25rem_minmax(4rem,0.9fr)_8.5rem_6rem_4.5rem]'
				: 'lg:grid-cols-[2rem_minmax(9rem,1.05fr)_6.5rem_3.25rem_minmax(5rem,1.3fr)_6.5rem_5.125rem]',
		)}
	>
		<span
			class={cn('font-mono text-[0.75rem]', row.isOurs ? 'text-link' : 'text-muted-foreground')}
		>
			{rank}
		</span>

		<span class="min-w-0">
			<span class="flex flex-wrap items-baseline gap-x-2 gap-y-1">
				<span class={cn('text-lead font-medium', row.isOurs && 'text-link')}>
					{display.name}
				</span>
				<ApiTag api={display.api} />
			</span>
			{#if display.note}
				<span class="text-meta text-muted-foreground mt-1 block">{display.note}</span>
			{/if}
			<!-- The database and the machine have their own columns from `lg`; below that they join the
			     note line rather than claiming columns the row cannot spare. -->
			<span class="mt-1 flex items-center gap-x-2 lg:hidden">
				<OsBadge os={row.summary.runner_os} detail={shard} />
				<span class="text-meta text-muted-foreground" title={dbDetail}>{db}</span>
			</span>
		</span>

		<!--
			`title` rather than a `Hint` here, deliberately. The two badges in this row are
			abbreviations and have to be explainable from the keyboard, so they are real tooltip
			triggers; "SQLite" is a word that already says what it is, and its family description is
			background. Making it a third focusable control per row would have tripled the tab stops
			needed to walk a twenty-row table for a sentence nobody is looking for.
		-->
		<span class="text-meta text-foreground-secondary max-lg:hidden" title={dbDetail}>{db}</span>

		<span class="max-lg:hidden">
			<OsBadge os={row.summary.runner_os} detail={shard} />
		</span>

		<!--
			Decorative: the number beside it is the value, and the bar is scaled within the current
			filter, so it says "relative to what is on screen" and nothing more. A row the sorted
			column never measured gets an empty track — a zero-width bar would read as a measured
			zero, and there is nothing here to draw.
		-->
		<span class="bg-muted block h-2 max-lg:hidden" aria-hidden="true">
			{#if row.barPct}
				<span
					class={cn(
						'block h-2',
						row.isOurs ? 'bg-primary' : 'bg-series-2',
						// A lower bound is drawn faint and open-ended: the value it represents is a floor,
						// so a solid bar ending at a definite point would overstate it.
						row.barKind === 'bound' &&
							'[mask-image:repeating-linear-gradient(90deg,#000_0_4px,transparent_4px_7px)] opacity-45',
					)}
					style="width:{row.barPct}"
				></span>
			{/if}
		</span>

		<!--
			`lg:contents` dissolves this wrapper into the grid on wide screens, so the numbers become
			their own columns. Below that they stack in one right-hand column, which is what keeps the
			row inside a 375px viewport without a horizontal scrollbar.
		-->
		<span class="text-right lg:contents">
			{#if showCapacity}
				<!-- Peak throughput always arrives with its objective attached; `CapacityFigure` is the
				     only thing on the site that can draw it, so it cannot arrive without. -->
				<CapacityFigure capacity={row.capacity} align="right" active={sort === 'capacity'} />
			{/if}
			<span
				class={cn(
					'text-lead block font-mono font-medium tabular-nums lg:text-right',
					sort === 'throughput' ? 'text-foreground' : 'text-foreground-secondary',
				)}
			>
				{fmtRps(p.rps.avg)}
				{#if showCapacity}
					<span class="text-meta text-muted-foreground mt-1 block font-sans font-normal">
						at fixed load
					</span>
				{/if}
			</span>
			<span
				class={cn(
					'text-foreground-secondary text-meta lg:text-body block font-mono tabular-nums lg:text-right',
					sort === 'latency' && 'text-foreground',
				)}
			>
				{fmtLatency(p.latency.p95)}
			</span>
		</span>
	</summary>

	<div class="bg-surface-inset border-border-soft mx-5 mb-5 border-t px-4 py-5 lg:mx-6 lg:px-5">
		<dl
			class="grid grid-cols-[repeat(auto-fit,minmax(8.5rem,1fr))] gap-x-6 gap-y-5 max-lg:grid-cols-2"
		>
			<div class="lg:hidden">
				<dt class="text-micro text-muted-foreground font-mono uppercase">database</dt>
				<dd class="text-body mt-1.5" title={dbDetail}>{db}</dd>
			</div>
			{#if showCapacity}
				<div class="lg:hidden">
					<dt class="text-micro text-muted-foreground font-mono uppercase">peak throughput</dt>
					<dd class="mt-1.5"><CapacityFigure capacity={row.capacity} /></dd>
				</div>
			{/if}
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">typical latency</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums">{fmtLatency(p.latency.avg)}</dd>
			</div>
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">p99</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums">{fmtLatency(p.latency.p99)}</dd>
			</div>
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">cpu</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums">{fmtCpu(p.cpu.avg)}</dd>
				<dd class="text-meta text-muted-foreground mt-1">peak core {fmtCpu(p.cpu.peak)}</dd>
			</div>
			{#if p.mem}
				<div>
					<dt class="text-micro text-muted-foreground font-mono uppercase">memory</dt>
					<dd class="text-body mt-1.5 font-mono tabular-nums">{p.mem.avg.toFixed(1)}MB</dd>
					<dd class="text-meta text-muted-foreground mt-1">peak {p.mem.peak.toFixed(1)}MB</dd>
				</div>
			{/if}
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">errors</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums">{fmtPct(p.err)}</dd>
			</div>
			<div>
				<!--
					Renamed from "peak throughput", which now means the saturation suite's capacity figure.
					This is the fastest single sample bucket of a paced run — a different fact, and one
					that must not borrow the other's name.
				-->
				<dt class="text-micro text-muted-foreground font-mono uppercase">
					<Hint
						hint="The fastest single sample bucket of the paced run. It is a momentary rate inside a fixed-load run, not a capacity figure."
					>
						busiest second
					</Hint>
				</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums">{fmtRps(p.rps.peak)}</dd>
			</div>
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">across trials</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums" title={spreadDetail}>{spread}</dd>
			</div>
			<div>
				<!--
					The scope is in the label, not only in the tooltip. One global table means the reference
					has to be named on the row: "vs drizzle-rs" alone reads as a comparison against every
					drizzle row in the set, which across databases would be a stack comparison wearing a
					library comparison's name.
				-->
				<dt class="text-micro text-muted-foreground font-mono uppercase">
					<Hint
						hint="This library against drizzle-rs on {db}, on throughput at fixed load. Both rows ran under the same harness, so the difference is attributable to the library. Positive means this library is faster. A dash means this set has no drizzle-rs row on {db}."
					>
						vs drizzle-rs on {db}
					</Hint>
				</dt>
				<dd class="text-body mt-1.5">
					<Delta text={row.deltaText} direction={row.deltaDirection} hint={row.deltaTitle} />
				</dd>
			</div>
			{#if harness}
				<div>
					<dt class="text-micro text-muted-foreground font-mono uppercase">
						<Hint hint={harness.detail}>harness</Hint>
					</dt>
					<dd class="text-body mt-1.5">
						{#if harness.summary}
							<span class="font-mono">{harness.summary}</span>
							{#if harness.identical === false}
								<span class="text-meta text-negative mt-1 block">not identical within family</span>
							{:else}
								<span class="text-meta text-muted-foreground mt-1 block">
									shared by every {db} row
								</span>
							{/if}
						{:else}
							<span class="text-muted-foreground italic">not declared</span>
						{/if}
					</dd>
				</div>
			{/if}
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">machine</dt>
				<dd class="text-body mt-1.5">
					<a class="text-link hover:underline" href="/runs/{row.summary.run_id}">
						{shardLabel(row.summary.runner_os, row.summary.run_id)}
					</a>
				</dd>
			</div>
		</dl>

		<!--
			Qualitative attributes are a footnote under the grid, never a cell in it. A sentence in a
			metric column wraps to five lines beside the numbers and reads as if it were one of them.
		-->
		{#if variant}
			<p class="border-border-soft text-meta text-muted-foreground mt-5 border-t pt-3">
				<span class="text-foreground-secondary">SQL:</span>
				{#if variant.elided}
					<Hint hint={variant.full}>{variant.short}</Hint>
				{:else}
					{variant.short}
				{/if}
			</p>
		{/if}
	</div>
</details>
