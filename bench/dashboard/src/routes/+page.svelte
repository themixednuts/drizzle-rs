<script lang="ts">
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import FilterPills from '#lib/components/FilterPills.svelte';
	import SortLinks from '#lib/components/SortLinks.svelte';
	import WarningNotice from '#lib/components/WarningNotice.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import RankRow from '#lib/components/RankRow.svelte';
	import RailAxis from '#lib/components/RailAxis.svelte';
	import Hint from '#lib/components/Hint.svelte';
	import ScopePlot from '#lib/components/ScopePlot.svelte';
	import Replay from '#lib/components/Replay.svelte';
	import HarnessStrip from '#lib/components/HarnessStrip.svelte';
	import RunList from '#lib/components/RunList.svelte';
	import Section from '#lib/components/Section.svelte';
	import { cn } from '#lib/utils.js';
	import { RunsPageState } from './home.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunsPageState(() => data);

	/** Shared by the header and every row, so a column and its label cannot drift apart. */
	const COLUMNS = $derived(
		view.hasCapacity
			? 'lg:grid-cols-[1.75rem_minmax(8rem,1fr)_4.5rem_minmax(7rem,1.3fr)_7rem_5rem_6rem_4.5rem]'
			: 'lg:grid-cols-[1.75rem_minmax(10rem,1fr)_minmax(10rem,1.9fr)_5.5rem_6rem_5rem]',
	);
</script>

<svelte:head>
	<title>drizzle-rs benchmarks</title>
</svelte:head>

<Page>
	<PageHeader title="Benchmarks">
		{#snippet subtitle()}{view.overviewMeta}{/snippet}
	</PageHeader>

	{#if view.warnings.length > 0}
		<div class="mt-6"><WarningNotice warnings={view.warnings} /></div>
	{/if}

	{#if !view.hasData}
		<div class="mt-7">
			<EmptyState title="No benchmark data published yet." />
		</div>
	{:else if view.results.length === 0}
		<div class="mt-7">
			<EmptyState title="No successful run summaries." />
		</div>
	{:else}
		<!-- The two groupings: the machine, then the SQL dialect. Everything below follows them. -->
		<div class="mt-6 flex flex-wrap items-center gap-x-4 gap-y-3">
			{#if view.osFilters.length > 1}
				<FilterPills label="os" options={view.osFilters} segmented />
			{/if}
			{#if view.dialectFilters.length > 2}
				<FilterPills label="dialect" options={view.dialectFilters} segmented />
			{/if}
			<SortLinks options={view.sortOptions} />
		</div>

		{#if !view.hasRankingRows}
			<div class="mt-5">
				<EmptyState
					title="No targets for this dialect on {view.osScope?.label ?? 'this platform'}."
				>
					<a class="underline" href={view.rankingUrl(null, view.sort)}>Show all</a>
				</EmptyState>
			</div>
		{:else}
			<section class="bg-card mt-4 rounded-md px-5 pt-5 pb-4 lg:px-6" aria-label="rate vs p95">
				<ScopePlot scope={view.scope} bind:hovered={view.hoverRowId} />
			</section>

			<div class="bg-card mt-4 rounded-md">
				<div
					class={cn(
						'bg-muted text-micro text-muted-foreground sticky top-0 z-10 grid grid-cols-[minmax(0,1fr)_auto] items-end gap-x-4 rounded-t-md px-5 pt-3 pb-2.5 font-mono sm:top-14 lg:gap-x-5 lg:px-6',
						COLUMNS,
					)}
				>
					<span class="pb-0.5 max-lg:hidden">#</span>
					<span class="pb-0.5">library</span>
					{#if view.hasCapacity}
						<span class="pb-0.5 max-lg:hidden">ramp</span>
					{/if}
					<span class="max-lg:hidden">
						<RailAxis rail={view.rail} />
					</span>
					{#if view.hasCapacity}
						<span class="pb-0.5 text-right max-lg:hidden">
							<Hint hint="Fastest step of the unpaced ramp that met the latency objective.">
								peak
							</Hint>
							{#if view.capacityObjective}<span class="text-foreground-faint normal-case"
									>{view.capacityObjective}</span
								>{/if}
						</span>
						<span class="pb-0.5 text-right max-lg:hidden">
							<Hint hint="Requests per second under the paced load.">paced</Hint>
						</span>
						<span class="pb-0.5 text-right lg:hidden">peak / paced / p95 / vs #1</span>
					{:else}
						<span class="pb-0.5 text-right max-lg:hidden">req/s</span>
						<span class="pb-0.5 text-right lg:hidden">req/s / p95 / vs #1</span>
					{/if}
					<span class="pb-0.5 text-right max-lg:hidden">
						{#if view.latencyBasis === 'sustained'}
							<Hint hint="p95 at the same offered load for every row.">
								{#if view.latencyLoad}p95 @ {view.latencyLoad} VUs{:else}p95 at load{/if}
							</Hint>
						{:else}
							<Hint hint="p95 across the whole ramp, queueing included.">p95 ramp</Hint>
						{/if}
					</span>
					<span class="pb-0.5 text-right max-lg:hidden">
						<Hint hint="Distance to the leader; below it, to the row above.">vs #1</Hint>
					</span>
				</div>

				{#each view.rankingRows as row (row.id)}
					<RankRow
						{row}
						display={view.targetDisplay(row.summary)}
						db={view.dbName(row.summary)}
						dbDetail={view.dbDetail(row.summary)}
						spread={view.throughputSummaryLabel(row.summary)}
						spreadDetail={view.throughputLabel(row.summary)}
						spreadBox={view.spreadFigure(row.summary)}
						latency={view.latency(row.summary)}
						showLatencyLoad={view.latencyLoad === null}
						variant={view.variantNote(row.summary)}
						harness={view.harnessFor(row.summary)}
						arch={view.architecture(row.summary)}
						sort={view.sort}
						showCapacity={view.hasCapacity}
						showRamp={view.hasCapacity}
						columns={COLUMNS}
						bind:hovered={view.hoverRowId}
					/>
				{/each}
			</div>

			<HarnessStrip rows={view.harnessRows} />

			{#if view.replay}
				<section class="bg-card mt-4 rounded-md px-5 pt-5 pb-5 lg:px-6" aria-label="load ramp">
					<Replay replay={view.replay} />
				</section>
			{/if}
		{/if}
	{/if}

	<Section title="Recent runs">
		{#snippet aside()}
			<a class="hover:text-foreground underline" href="/runs">all {view.totalCohorts}</a>
		{/snippet}

		<RunList cohorts={view.recentCohorts}>
			{#snippet empty()}
				<EmptyState title="No runs match." />
			{/snippet}
		</RunList>
	</Section>
</Page>
