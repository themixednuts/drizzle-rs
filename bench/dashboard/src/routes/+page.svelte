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
	import FamilyDivider from '#lib/components/FamilyDivider.svelte';
	import VerdictStrip from '#lib/components/VerdictStrip.svelte';
	import RunList from '#lib/components/RunList.svelte';
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
		-->
		<div class="mt-7 flex flex-wrap items-center gap-x-4 gap-y-3">
			<FilterPills label="database" options={view.dbFilters} />
			<SortLinks options={view.sortOptions} />
		</div>

		{#if !view.hasFamilies}
			<div class="mt-7">
				<EmptyState title="No results for this database.">
					This set published no targets for that database. Choose
					<a class="text-link underline" href={view.rankingUrl(null, view.sort)}>All</a>.
				</EmptyState>
			</div>
		{:else}
			<VerdictStrip verdicts={view.verdicts} />

			<!--
				One table for every family. The column header is sticky, so the meaning of a column
				survives scrolling past a divider; each family is a `role="group"` labelled by its
				divider, and rank/bar/delta are all computed inside the band.
			-->
			<div class="bg-card border-border mt-4 border">
				<div
					class="bg-surface-raised border-border text-micro text-muted-foreground sticky top-0 z-10 grid grid-cols-[1.75rem_minmax(0,1fr)_auto] items-center gap-x-3 border-b px-5 py-3 font-mono uppercase sm:top-[3.875rem] lg:grid-cols-[2rem_minmax(9rem,1.05fr)_6.5rem_minmax(7rem,1.5fr)_6.5rem_5.125rem] lg:gap-x-6 lg:px-6 lg:py-3.5"
				>
					<span><span class="sr-only">rank</span></span>
					<span>library</span>
					<span class="max-lg:hidden">database</span>
					<span class="max-lg:hidden"><span class="sr-only">relative throughput</span></span>
					<span class="text-right max-lg:hidden">requests/sec</span>
					<span class="text-right lg:hidden">rps / p95</span>
					<span class="text-right max-lg:hidden">p95</span>
				</div>

				{#each view.rankingFamilies as family (family.key)}
					<FamilyDivider {family} />
					<div role="group" aria-labelledby="{family.anchor}-label">
						{#each family.rows as row (row.id)}
							<RankRow
								{row}
								display={view.targetDisplay(row.summary)}
								db={view.dbName(row.summary)}
								dbDetail={view.dbDetail(row.summary)}
								spread={view.throughputSummaryLabel(row.summary)}
								spreadDetail={view.throughputLabel(row.summary)}
								variant={view.variantNote(row.summary)}
								ranked={family.ranked}
								sort={view.sort}
							/>
						{/each}
					</div>
				{/each}
			</div>

			<div class="mt-4">
				<Note>
					Rank, bar and delta are all measured inside a database. Across databases the jobs ran on
					<a class="text-link underline" href="/repeatability">different machines</a>, and a repeat
					of the same job can land far apart. Open a row for the full numbers, or read
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
