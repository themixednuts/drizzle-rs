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
	} = $props();

	const p = $derived(row.summary.primary);
	const rank = $derived(String(row.rank).padStart(2, '0'));
</script>

<details
	class={cn('group border-border-soft border-b last:border-b-0', row.isOurs && 'bg-accent-tint')}
>
	<summary
		class="hover:bg-muted/40 grid cursor-pointer list-none grid-cols-[2rem_minmax(9rem,1.05fr)_6.5rem_minmax(7rem,1.5fr)_6.5rem_5.125rem] items-center gap-x-6 px-6 py-4 transition-colors marker:content-[''] max-lg:grid-cols-[2rem_minmax(9rem,1fr)_6.5rem_5.125rem]"
	>
		<span
			class={cn('font-mono text-[0.75rem]', row.isOurs ? 'text-link' : 'text-muted-foreground')}
		>
			{rank}
		</span>

		<span class="min-w-0">
			<span class={cn('text-lead block font-semibold', row.isOurs && 'text-link')}>
				{display.name}
			</span>
			{#if display.note}
				<span class="text-meta text-muted-foreground mt-1 block">{display.note}</span>
			{/if}
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

		<span
			class={cn(
				'text-lead text-right font-mono font-medium tabular-nums',
				sort === 'throughput' && 'text-foreground',
			)}
		>
			{fmtRps(p.rps.avg)}
		</span>
		<span
			class={cn(
				'text-foreground-secondary text-body text-right font-mono tabular-nums',
				sort === 'latency' && 'text-foreground',
			)}
		>
			{fmtLatency(p.latency.p95)}
		</span>
	</summary>

	<div class="bg-surface-inset border-border-soft mx-6 mb-5 border-t px-5 py-5">
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
				<dt class="text-micro text-muted-foreground font-mono uppercase">vs drizzle-rs</dt>
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
