<script lang="ts">
	import ApiTag from './ApiTag.svelte';
	import RailMark from './RailMark.svelte';
	import RampSpark from './RampSpark.svelte';
	import BoxWhisker from './BoxWhisker.svelte';
	import CapacityFigure from './CapacityFigure.svelte';
	import Hint from './Hint.svelte';
	import type { BoxWhiskerDatum, BoxWhiskerExtent } from '#lib/boxplot';
	import { cn } from '#lib/utils.js';
	import { fmtCpu, fmtLatency, fmtPct, fmtRps, shardLabel } from '#lib/format';
	import type { HarnessRow } from '#lib/harness';
	import type { QualitativeNote } from '#lib/qualitative';
	import type { RankingRow, RankingSort } from '#lib/ranking';
	import type { LatencyView } from '#lib/service-latency';
	import type { TargetDisplay } from '#lib/target-display';

	/**
	 * One row of the ranking, read like a line on a timing board.
	 *
	 * Position, entrant, the shape of its ramp, where it sits on the shared ratio rail, its figures,
	 * and two distances: to the top of the table, and to the row directly above. Nothing on the row
	 * is measured against a nominated target — a board that measured every entrant against the one
	 * its author entered would be answering a different question than it appears to be asking.
	 *
	 * The two distances do different work and both are needed. The gap says how far off the pace this
	 * row is; the interval is where the clusters show, because four rows within a percent of each
	 * other are one result however far all four are from the top.
	 *
	 * There is no `os` column: the ranking is always scoped to one operating system, so the badge was
	 * the same on all twenty-seven rows. It is stated once under the table. The database moved onto
	 * the target's own note line for the same reason — five repeating words do not need a column.
	 *
	 * A native `<details>` rather than a JS disclosure: it is keyboard operable, announces its own
	 * expanded state, and works with scripting off.
	 */
	let {
		row,
		display,
		db,
		dbDetail,
		spread,
		spreadDetail,
		spreadBox,
		latency,
		sort,
		columns,
		variant = null,
		harness = null,
		showCapacity = false,
		showRamp = false,
		showLatencyLoad = true,
		hovered = $bindable(null),
	}: {
		row: RankingRow;
		display: TargetDisplay;
		db: string;
		dbDetail: string;
		spread: string;
		spreadDetail: string;
		/** The same trials as `spread`, as a shape: min, quartiles where recorded, and max. */
		spreadBox: { box: BoxWhiskerDatum; extent: BoxWhiskerExtent };
		/**
		 * The row's latency on the table's basis. Passed in rather than derived here, because which
		 * of the two latencies a row may show is a property of the table, not of the row.
		 */
		latency: LatencyView;
		sort: RankingSort;
		/**
		 * The table's grid template, passed down from the page so the header and the rows cannot
		 * drift apart. The rail column in particular has to be exactly as wide here as it is under
		 * the axis, or every mark on the page is offset from the ticks it is measured against.
		 */
		columns: string;
		/** Short form plus full text for the target's SQL notes; `null` when it declared none. */
		variant?: QualitativeNote | null;
		/** The harness this row's whole database ran under; `null` when nothing was declared. */
		harness?: HarnessRow | null;
		/** Whether this set measured capacity at all — see `RunsPageState.hasCapacity`. */
		showCapacity?: boolean;
		/**
		 * Whether the set recorded ramps. A column of empty cells under a "ramp" heading reads as a
		 * measurement that came back blank rather than one that was never taken, so the column is left
		 * off entirely when nothing in the set has one.
		 */
		showRamp?: boolean;
		/**
		 * Whether the row prints the load its latency was read at.
		 *
		 * False in the normal case, where every row is read at the same load and the heading says so
		 * once. True only when a table spans ladders that disagree, and each row has to carry its own.
		 */
		showLatencyLoad?: boolean;
		/** The row under the pointer anywhere on the page, shared with the plot above the table. */
		hovered?: string | null;
	} = $props();

	const p = $derived(row.summary.primary);
	/** Position, for the accessible name only. `null` when the sorted column never measured it. */
	const place = $derived(row.rank === null ? 'unranked' : `number ${row.rank}`);
	/** True when this row is lit from the plot rather than from the pointer being on it. */
	const linked = $derived(hovered === row.id);
</script>

<details
	id="rank-{row.id}"
	class={cn(
		'group border-border-soft border-b transition-colors last:border-b-0',
		// Identity: this row is drizzle-rs. Deliberately faint — see `--signal-wash` in app.css.
		row.isOurs && 'bg-signal-wash',
		// Attention: the row under the pointer, holding keyboard focus, or lit from the plot above,
		// always wins. `hover:` and `focus-within:` carry a pseudo-class, so they out-specify the
		// identity tint above on the drizzle row too.
		'hover:bg-signal-wash-strong focus-within:bg-signal-wash-strong',
		linked && 'bg-signal-wash-strong',
	)}
	onmouseenter={() => (hovered = row.id)}
	onmouseleave={() => (hovered = null)}
>
	<summary
		class={cn(
			"grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 px-5 py-3.5 transition-colors marker:content-[''] lg:gap-x-5 lg:px-6",
			columns,
		)}
	>
		<!--
			Position. It left this table once, on the argument that a sorted table already shows it —
			which is true right up until a reader wants to say which row they mean, or count how far
			down something sits. Rows the sorted column never measured carry no number at all rather
			than one that would read as a placement.
		-->
		<span
			class="text-meta text-muted-foreground font-mono tabular-nums max-lg:hidden"
			aria-hidden="true"
		>
			{row.rank ?? '—'}
		</span>

		<span class="min-w-0">
			<span class="sr-only">{place}, </span>
			<span class="flex flex-wrap items-baseline gap-x-2 gap-y-1">
				<span class={cn('text-lead font-medium', row.isOurs && 'text-signal-ink')}>
					{display.name}
				</span>
				<ApiTag api={display.api} />
			</span>
			<!--
				The engine leads the note line, because it is the fact that decides whether two rows are
				comparable at all, and it used to be a column of repeating words.
			-->
			<span class="text-meta text-muted-foreground mt-1 block">
				<span class="text-foreground-secondary" title={dbDetail}>{db}</span>{#if display.note}
					· {display.note}{/if}
			</span>
		</span>

		<!-- The run behind the number: whether the ramp flattened, kept climbing, or turned over. -->
		{#if showRamp}
			<span class="block max-lg:hidden">
				{#if row.ramp}
					<RampSpark ramp={row.ramp} />
				{/if}
			</span>
		{/if}

		<!--
			The row's position on the shared ratio rail. The axis is drawn once in the table header,
			so this is a mark and not a scale of its own.
		-->
		<span class="block max-lg:hidden">
			<RailMark left={row.railLeft} ours={row.isOurs} kind={row.barKind} />
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
				<CapacityFigure
					capacity={row.capacity}
					align="right"
					active={sort === 'capacity'}
					showQualifier={false}
				/>
			{/if}
			<span
				class={cn(
					'text-lead block font-mono font-medium tabular-nums lg:text-right',
					sort === 'throughput' ? 'text-foreground' : 'text-foreground-secondary',
				)}
			>
				{fmtRps(p.rps.avg)}
			</span>
			<!--
				Latency, with the load it was read at underneath. The qualifier is not decoration: the
				same column carries two different measurements depending on what the run recorded, and
				a bare figure would not say which one this is.
			-->
			<span class="block lg:text-right" title={latency.detail}>
				<span
					class={cn(
						'text-foreground-secondary text-meta lg:text-body block font-mono tabular-nums',
						sort === 'latency' && 'text-foreground',
					)}
				>
					{latency.text}
				</span>
				{#if showLatencyLoad || latency.basis === 'whole-ramp'}
					<span class="text-micro text-muted-foreground mt-0.5 block font-mono">
						{latency.note}
					</span>
				{/if}
			</span>
			<!--
				Two distances in one column: to the top of the table, and under it to the row directly
				above. Both are on whichever column the table is ordered by, so they never describe a
				different measurement than the one the reader chose to sort on.
			-->
			<span class="block lg:text-right">
				<span
					class="text-meta lg:text-body text-foreground-secondary block font-mono tabular-nums"
					title={row.gapTitle}
				>
					{row.gapText}
				</span>
				<span
					class="text-micro text-muted-foreground mt-0.5 block font-mono tabular-nums"
					title={row.intervalTitle}
				>
					{row.intervalText}
				</span>
			</span>
		</span>
	</summary>

	<!--
		What is left after the row itself answers the common questions: the numbers a reader opens a
		row *for*, rather than every number the artifact happens to carry. Mean and p99 latency, cpu,
		memory, errors, the across-trial spread that says whether to believe any of it, and the shard
		it came off. Six fields, down from ten — `database`, `busiest second` and the delta all moved
		out, the first two onto the row and the third into its own column.
	-->
	<div class="bg-surface-inset border-border-soft mx-5 mb-5 border-t px-4 py-5 lg:mx-6 lg:px-5">
		<dl
			class="grid grid-cols-[repeat(auto-fit,minmax(8.5rem,1fr))] gap-x-6 gap-y-5 max-lg:grid-cols-2"
		>
			{#if showCapacity}
				<div class="lg:hidden">
					<dt class="text-micro text-muted-foreground font-mono uppercase">peak throughput</dt>
					<dd class="mt-1.5"><CapacityFigure capacity={row.capacity} /></dd>
				</div>
			{/if}
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">
					<Hint
						hint="Median across trials of the mean latency inside each trial. The p95 on the row is the one to compare; this says where the bulk of requests sat."
					>
						typical latency
					</Hint>
				</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums">{fmtLatency(p.latency.avg)}</dd>
				<dd class="text-meta text-muted-foreground mt-1">p99 {fmtLatency(p.latency.p99)}</dd>
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
				<dt class="text-micro text-muted-foreground font-mono uppercase">busiest second</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums">{fmtRps(p.rps.peak)}</dd>
			</div>
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">
					{#if harness}
						<Hint hint={harness.detail}>ran under</Hint>
					{:else}
						ran under
					{/if}
				</dt>
				<dd class="text-body mt-1.5">
					<a class="underline underline-offset-2" href="/runs/{row.summary.run_id}">
						{shardLabel(row.summary.runner_os, row.summary.run_id)}
					</a>
				</dd>
				{#if harness?.summary}
					<dd class="text-meta text-muted-foreground mt-1 font-mono">
						{harness.summary}
						{#if harness.identical === false}
							<span class="text-negative">· not identical within family</span>
						{/if}
					</dd>
				{/if}
			</div>
		</dl>

		<!--
			The five trials behind the one number on the row, drawn rather than described.

			It gets a full-width block instead of a cell in the grid above because it is the field a
			reader opens a row to check: where a row's trials range wider than its interval to the row
			above, those two rows are not separated by this measurement, and no figure in the grid can
			say that. Whiskers reach the slowest and fastest trial; the box spans the middle two
			quartiles, and is drawn only where the run recorded them.
		-->
		<div class="border-border-soft mt-5 border-t pt-4">
			<div class="text-micro text-muted-foreground font-mono uppercase">
				<Hint hint={spreadDetail}>rate across trials</Hint>
			</div>
			<div class="mt-2 grid items-center gap-x-5 gap-y-2 lg:grid-cols-[minmax(0,1fr)_auto]">
				<BoxWhisker box={spreadBox.box} extent={spreadBox.extent} label={spreadDetail} />
				<span class="text-meta text-foreground-secondary font-mono tabular-nums">{spread}</span>
			</div>
		</div>

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
