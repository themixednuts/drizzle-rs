<script lang="ts">
	import Delta from './Delta.svelte';
	import Hint from './Hint.svelte';
	import { cn } from '#lib/utils.js';
	import { fmtCpu, fmtLatency, fmtPct, fmtRps, shardLabel } from '#lib/format';
	import type { QualitativeNote } from '#lib/qualitative';
	import type { RankingRow, RankingSort } from '#lib/ranking';
	import type { TargetDisplay } from '#lib/target-display';

	/**
	 * One row of the flat ranking.
	 *
	 * Closed, it is exactly the comp's six columns: rank, library with its note, database, a bar,
	 * throughput, p95. Opened, it is every other number the old sectioned table used to show
	 * inline — mean and p99 latency, cpu, errors, memory, the across-trial spread, and the delta
	 * against drizzle-rs.
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
		ranked = true,
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
		/**
		 * False for the in-process-cache band, which is listed but carries no rank: it is not doing
		 * the same work as the rest, so numbering it against them would be the claim this whole
		 * layout exists to avoid.
		 */
		ranked?: boolean;
	} = $props();

	const p = $derived(row.summary.primary);
	const rank = $derived(String(row.rank).padStart(2, '0'));
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
		class="grid cursor-pointer list-none grid-cols-[1.75rem_minmax(0,1fr)_auto] items-center gap-x-3 px-5 py-4 transition-colors marker:content-[''] lg:grid-cols-[2rem_minmax(9rem,1.05fr)_6.5rem_minmax(7rem,1.5fr)_6.5rem_5.125rem] lg:gap-x-6 lg:px-6"
	>
		<span
			class={cn('font-mono text-[0.75rem]', row.isOurs ? 'text-link' : 'text-muted-foreground')}
		>
			{#if ranked}{rank}{:else}<span aria-hidden="true">·</span>{/if}
		</span>

		<span class="min-w-0">
			<span class={cn('text-lead block font-medium', row.isOurs && 'text-link')}>
				{display.name}
			</span>
			{#if display.note}
				<span class="text-meta text-muted-foreground mt-1 block">{display.note}</span>
			{/if}
			<!-- The database has its own column from `lg`; below that it joins the note line rather
			     than claiming a column the row cannot spare. -->
			<span class="text-meta text-muted-foreground mt-0.5 block lg:hidden" title={dbDetail}>
				{db}
			</span>
		</span>

		<span class="text-meta text-foreground-secondary max-lg:hidden" title={dbDetail}>{db}</span>

		<!-- Decorative: the number beside it is the value, and the bar is scaled within the current
		     filter, so it says "relative to what is on screen" and nothing more. -->
		<span class="bg-muted block h-2 max-lg:hidden" aria-hidden="true">
			<span
				class={cn('block h-2', row.isOurs ? 'bg-primary' : 'bg-series-2')}
				style="width:{row.barPct}"
			></span>
		</span>

		<!--
			`lg:contents` dissolves this wrapper into the grid on wide screens, so the two numbers
			become their own columns. Below that they stack in one right-hand column, which is what
			keeps the row inside a 375px viewport without a horizontal scrollbar.
		-->
		<span class="text-right lg:contents">
			<span
				class={cn(
					'text-lead block font-mono font-medium tabular-nums lg:text-right',
					sort === 'throughput' && 'text-foreground',
				)}
			>
				{fmtRps(p.rps.avg)}
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
				<dt class="text-micro text-muted-foreground font-mono uppercase">peak throughput</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums">{fmtRps(p.rps.peak)}</dd>
			</div>
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">across trials</dt>
				<dd class="text-body mt-1.5 font-mono tabular-nums" title={spreadDetail}>{spread}</dd>
			</div>
			<div>
				<dt class="text-micro text-muted-foreground font-mono uppercase">
					<Hint
						hint="This library against drizzle-rs on throughput. Positive means this library is faster."
					>
						vs drizzle-rs
					</Hint>
				</dt>
				<dd class="text-body mt-1.5">
					<Delta text={row.deltaText} direction={row.deltaDirection} hint={row.deltaTitle} />
				</dd>
			</div>
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
