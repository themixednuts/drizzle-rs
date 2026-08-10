<script lang="ts">
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import FilterPills from '#lib/components/FilterPills.svelte';
	import SortLinks from '#lib/components/SortLinks.svelte';
	import WarningNotice from '#lib/components/WarningNotice.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Note from '#lib/components/Note.svelte';
	import RankRow from '#lib/components/RankRow.svelte';
	import VerdictStrip from '#lib/components/VerdictStrip.svelte';
	import HarnessStrip from '#lib/components/HarnessStrip.svelte';
	import RunList from '#lib/components/RunList.svelte';
	import { cn } from '#lib/utils.js';
	import { RunsPageState } from './home.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunsPageState(() => data);
</script>

<svelte:head>
	<title>Ranking / drizzle-rs benchmarks</title>
</svelte:head>

<Page>
	<PageHeader title="Ranking">
		{#snippet subtitle()}{view.overviewMeta}{/snippet}
	</PageHeader>

	{#if view.warnings.length > 0}
		<div class="mt-7"><WarningNotice warnings={view.warnings} /></div>
	{/if}

	{#if !view.hasData}
		<div class="mt-7">
			<EmptyState title="No benchmark data has been published yet.">
				Publish a run to the <code class="font-mono">BENCH_DATA</code> bucket, or point
				<code class="font-mono">BENCH_DATA_DIR</code> at a local
				<code class="font-mono">dashboard-bench-data</code> export and reload.
			</EmptyState>
		</div>
	{:else if view.results.length === 0}
		<div class="mt-7">
			<EmptyState title="No successful run summaries are available.">
				A set appears here once at least one of its shards publishes target summaries.
			</EmptyState>
		</div>
	{:else}
		<!--
			The filters render whenever there is anything to filter, including when the current filter
			matches nothing. Hiding them in that state left the only way back to "All" in the URL bar.

			The OS pills come first because they scope everything after them: rank, bar, delta and the
			database pills themselves are all computed inside one operating system. A rank that spanned
			operating systems would be two comparisons stacked — different kernels, different CI
			machines, and a field that cannot even hold the same families, since GitHub runs service
			containers on Linux only.
		-->
		<div class="mt-7 flex flex-wrap items-center gap-x-4 gap-y-3">
			<FilterPills label="os" options={view.osFilters} />
			<FilterPills label="database" options={view.dbFilters} />
			<SortLinks options={view.sortOptions} />
		</div>

		{#if !view.hasRankingRows}
			<div class="mt-7">
				<EmptyState title="No results for this database.">
					This set published no targets for that database on {view.osScope?.label ??
						'this platform'}. Choose
					<a class="text-link underline" href={view.rankingUrl(null, view.sort)}>All</a>, or another
					platform above.
				</EmptyState>
			</div>
		{:else}
			<VerdictStrip verdicts={view.verdicts} />
			<HarnessStrip rows={view.harnessRows} />

			<!--
				One table, one order, every database. The column header is sticky, so the meaning of a
				column survives scrolling; rank runs 01..N across the whole list and the bar is scaled to
				the fastest row on screen. Which database a row ran against, and on which machine, are
				columns rather than section headings.

				When the set measured capacity, the peak-throughput column leads and the paced number
				keeps its own column headed "at fixed load", so the two throughput readings are never
				one column whose meaning changed.
			-->
			<div class="bg-card border-border mt-4 border">
				<div
					class={cn(
						'bg-surface-raised border-border text-micro text-muted-foreground sticky top-0 z-10 grid grid-cols-[1.75rem_minmax(0,1fr)_auto] items-center gap-x-3 border-b px-5 py-3 font-mono uppercase sm:top-[3.875rem] lg:gap-x-5 lg:px-6 lg:py-3.5',
						view.hasCapacity
							? 'lg:grid-cols-[2rem_minmax(8rem,1fr)_5.5rem_3.25rem_minmax(4rem,0.9fr)_8.5rem_6rem_4.5rem]'
							: 'lg:grid-cols-[2rem_minmax(9rem,1.05fr)_6.5rem_3.25rem_minmax(5rem,1.3fr)_6.5rem_5.125rem]',
					)}
				>
					<span><span class="sr-only">rank</span></span>
					<span>library</span>
					<span class="max-lg:hidden">database</span>
					<span class="max-lg:hidden">os</span>
					<span class="max-lg:hidden"><span class="sr-only">relative throughput</span></span>
					{#if view.hasCapacity}
						<span class="text-right max-lg:hidden">peak throughput</span>
						<span class="text-right max-lg:hidden">at fixed load</span>
						<span class="text-right lg:hidden">peak / rps / p95</span>
					{:else}
						<span class="text-right max-lg:hidden">requests/sec</span>
						<span class="text-right lg:hidden">rps / p95</span>
					{/if}
					<span class="text-right max-lg:hidden">p95</span>
				</div>

				{#each view.rankingRows as row (row.id)}
					<RankRow
						{row}
						display={view.targetDisplay(row.summary)}
						db={view.dbName(row.summary)}
						dbDetail={view.dbDetail(row.summary)}
						spread={view.throughputSummaryLabel(row.summary)}
						spreadDetail={view.throughputLabel(row.summary)}
						variant={view.variantNote(row.summary)}
						harness={view.harnessFor(row.summary)}
						sort={view.sort}
						showCapacity={view.hasCapacity}
					/>
				{/each}
			</div>

			<div class="mt-4 space-y-3">
				{#if view.hasCapacity}
					<Note>
						<strong class="text-foreground-secondary font-medium">Two throughput numbers.</strong>
						<em>Peak throughput</em> is the highest request rate a target sustained while still
						meeting the latency objective, found by ramping concurrency with no think time — it is a
						capacity figure and always carries the objective it was measured at.
						<em>At fixed load</em>
						is the paced suite's rate, where the generator offers a set amount of work; a healthy target
						reports the pacing ceiling rather than its capacity. Sorting by peak throughput puts every
						row without a measured peak below every row that has one, and those rows carry no rank number,
						because "we did not find out" is not a placement.
					</Note>
				{:else}
					<Note>
						<strong class="text-foreground-secondary font-medium">No peak throughput here.</strong>
						This set predates the saturation suite, so no target in it has a measured capacity figure
						and the peak-throughput column is left off rather than filled with a paced number wearing
						its name. The throughput below is the paced suite's rate at a fixed offered load — see
						<a class="text-link underline" href="/methodology">the method</a> for why that cannot be read
						as capacity.
					</Note>
				{/if}
				<Note>
					{#if view.osScope}
						<strong class="text-foreground-secondary font-medium"
							>One table, every database, one operating system.</strong
						>
						This ranking covers {view.osScope.label} only — {view.osScope.detail
							.charAt(0)
							.toLowerCase() + view.osScope.detail.slice(1)}
						{#if view.osScopes.length > 1}
							The other platforms are their own rankings, on the pills above; they are not folded
							in, because a rank across operating systems compares machines rather than libraries.
						{/if}
					{:else}
						One table, every database.
					{/if}
					Rows on different databases still did different amounts of work per request, and a
					<a class="text-link underline" href="/repeatability">repeat of the same job</a> can land
					far apart even on one platform. Open a row for the full numbers and how it compares to
					drizzle-rs on its own database, or read
					<a class="text-link underline" href="/methodology">the method</a>.
				</Note>
			</div>
		{/if}
	{/if}

	<Section title="Recent runs">
		{#snippet aside()}
			<a class="text-link hover:underline" href="/runs">all {view.totalCohorts}</a>
		{/snippet}

		<RunList cohorts={view.recentCohorts}>
			{#snippet empty()}
				<EmptyState title="No runs match the selected filters.">
					Try a different suite or status.
				</EmptyState>
			{/snippet}
		</RunList>
	</Section>
</Page>
