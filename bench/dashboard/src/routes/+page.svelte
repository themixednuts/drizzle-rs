<script lang="ts">
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
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
	import { cn } from '#lib/utils.js';
	import { RunsPageState } from './home.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunsPageState(() => data);

	/**
	 * The grid template the table uses, shared by the header and every row so a column and its label
	 * cannot drift apart. The rail takes the widest column available: a decade of logarithmic scale
	 * under about 240px stops reading as distance.
	 *
	 * Position, target, rail, figures, distance — plus the ramp and peak throughput on a set that
	 * measured a ramp. Everything but the target and the rail is a fixed width, because these are all
	 * monospaced figures read down a column and a column that resizes with its content stops being
	 * one.
	 */
	const COLUMNS = $derived(
		view.hasCapacity
			? 'lg:grid-cols-[1.75rem_minmax(8rem,1fr)_4.5rem_minmax(7rem,1.3fr)_7rem_5rem_6rem_4.5rem]'
			: 'lg:grid-cols-[1.75rem_minmax(10rem,1fr)_minmax(10rem,1.9fr)_5.5rem_6rem_5rem]',
	);
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
			The field, then the controls, then the table. The page used to run controls-first, which told
			a reader what they could adjust before telling them anything worth adjusting.

			The plot leads because it is the only view here that does not have to pick one column to be
			the order. The table below is that same set of rows put in one, and hovering either lights
			the other.
		-->
		<section
			class="bg-card mt-6 rounded-md px-5 pt-5 pb-4 lg:px-6"
			aria-label="throughput against tail latency"
		>
			<ScopePlot scope={view.scope} bind:hovered={view.hoverRowId} />
		</section>

		<!--
			The ramp the plot is a snapshot of.

			It goes here rather than below the table because it answers the first question the plot
			raises. Every point on that plot sits on a diagonal — the faster a target is, the lower its
			p95 — and that looks like a suspiciously tidy result until you watch the ramp: the load
			climbs to three thousand virtual users whether or not a target can serve them, so past its
			ceiling the extra load is queue depth and the queue is most of what the p95 measures.
		-->
		{#if view.replay}
			<section class="bg-card mt-4 rounded-md px-5 pt-5 pb-5 lg:px-6">
				<Replay replay={view.replay} />
				<p class="text-meta text-muted-foreground measure mt-4">
					Past a target's ceiling the extra load is queue time, which is most of the p95 column.
				</p>
			</section>
		{/if}

		<!--
			The filters sit directly above the table they filter.

			The OS pills come first because they scope everything after them: position, mark, distances
			and the database pills themselves are all computed inside one operating system. A rank that spanned
			operating systems would be two comparisons stacked — different kernels, different CI
			machines, and a field that cannot even hold the same families, since GitHub runs service
			containers on Linux only.

			They render whenever there is anything to filter, including when the current filter matches
			nothing. Hiding them in that state left the only way back to "All" in the URL bar.
		-->
		<div class="mt-10 flex flex-wrap items-center gap-x-5 gap-y-3">
			<FilterPills label="os" options={view.osFilters} />
			<FilterPills label="database" options={view.dbFilters} />
			<SortLinks options={view.sortOptions} />
		</div>

		{#if !view.hasRankingRows}
			<div class="mt-5">
				<EmptyState title="No results for this database.">
					This set published no targets for that database on {view.osScope?.label ??
						'this platform'}. Choose
					<a class="underline" href={view.rankingUrl(null, view.sort)}>All</a>, or another platform
					above.
				</EmptyState>
			</div>
		{:else}
			<!--
				One table, one order, every database, and one machine — the ranking is always scoped to a
				single operating system, which is why there is no `os` column: it would be the same badge
				on every row. The scope is stated once under the table.

				Every row's mark sits on one shared logarithmic rail, whose axis is drawn once in the
				header below and stays there while the table scrolls. That is what makes a flat table
				across five engines readable: equal distance is equal ratio, so the row serving from an
				in-process cache no longer sets the scale for every row doing real per-request work.
			-->
			<div class="bg-card mt-4 rounded-md">
				<div
					class={cn(
						'bg-muted text-micro text-muted-foreground type-narrow sticky top-0 z-10 grid grid-cols-[minmax(0,1fr)_auto] items-end gap-x-4 rounded-t-md px-5 pt-3 pb-2.5 font-mono uppercase sm:top-14 lg:gap-x-5 lg:px-6',
						COLUMNS,
					)}
				>
					<span class="pb-0.5 max-lg:hidden">pos</span>
					<span class="pb-0.5">library</span>
					{#if view.hasCapacity}
						<span class="pb-0.5 max-lg:hidden">ramp</span>
					{/if}
					<!-- The rail's axis: the one column header that is a scale rather than a word. -->
					<span class="max-lg:hidden">
						<RailAxis rail={view.rail} />
					</span>
					{#if view.hasCapacity}
						<!--
							The objective states itself once here, for the column, instead of once per row.

							This column and the two beside it come off different halves of the same CI run —
							one unpaced ramp on a single machine per platform, one paced ramp spread over
							several — so each says where it came from. The figures are never compared across
							that line; the row is a join, not an average.
						-->
						<span class="pb-0.5 text-right max-lg:hidden">
							<Hint
								hint="From the unpaced saturation ramp: concurrency stepped up until throughput stops rising. Every family runs back to back on one machine per platform, so these figures are comparable with each other. They are a different measurement from the paced columns beside them and are never compared against those."
							>
								peak throughput
							</Hint>
							{#if view.capacityObjective}<span class="text-foreground-faint normal-case"
									>{view.capacityObjective}</span
								>{/if}
						</span>
						<span class="pb-0.5 text-right max-lg:hidden">
							<Hint
								hint="From the paced ramp, where the generator offers a fixed load with think time. Families run on separate machines here, so a gap between two databases carries some of the hardware they landed on."
							>
								at fixed load
							</Hint>
						</span>
						<span class="pb-0.5 text-right lg:hidden">peak / rps / p95 / gap</span>
					{:else}
						<span class="pb-0.5 text-right max-lg:hidden">requests/sec</span>
						<span class="pb-0.5 text-right lg:hidden">rps / p95 / gap</span>
					{/if}
					<!-- The column carries one of two different measurements; the heading says which. -->
					<span class="pb-0.5 text-right max-lg:hidden">
						{#if view.latencyBasis === 'sustained'}
							<Hint
								hint="p95 at one offered load, the same for every row, chosen low enough on the ramp that every target still serves it in full. Past a target's ceiling a closed ramp adds queueing rather than work, so a figure read there would measure the queue. How far up the ramp each target went is in its own row."
							>
								{#if view.latencyLoad}p95 at {view.latencyLoad} VUs{:else}p95 at load{/if}
							</Hint>
						{:else}
							<Hint
								hint="p95 merged across the whole ramp, up to 3000 concurrent. Past a target's throughput ceiling every further second contributes queueing, so this ranks targets partly by how far the ramp overshot them. It is the upstream drizzle-benchmarks method, kept so the throughput figure stays comparable with theirs."
							>
								p95 whole ramp
							</Hint>
						{/if}
					</span>
					<!--
						Two distances under one heading: to the top of the table, and to the row above.
						Both on whichever column the table is sorted by.
					-->
					<span class="pb-0.5 text-right max-lg:hidden">
						<Hint
							hint="Top figure: distance to the row leading this order. Below it: distance to the row directly above, which is where the field's clusters show. Both are measured on the column the table is sorted by."
						>
							gap / int
						</Hint>
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
						sort={view.sort}
						showCapacity={view.hasCapacity}
						showRamp={view.hasCapacity}
						columns={COLUMNS}
						bind:hovered={view.hoverRowId}
					/>
				{/each}
			</div>

			<!--
				`{:else}` rather than two adjacent blocks: putting `{/if}` straight against the next word
				rendered "separately.Rows" with no space between the sentences.
			-->
			<p class="text-meta text-muted-foreground mt-3.5">
				{#if view.osScope}{view.osScope.label} only.{/if}
				Rows on different databases did different work per request, and the same job
				<a class="underline underline-offset-2" href="/runs/machines">on another machine</a>
				can land far apart.
			</p>

			<!--
				The run configuration is reference, not a finding: seven blocks of pragmas a reader
				consults once and never again. `HarnessStrip` is its own disclosure, closed by default,
				and it answers the one question worth asking without being opened — whether every group
				was internally consistent.
			-->
			<HarnessStrip rows={view.harnessRows} />
		{/if}
	{/if}

	<Section title="Recent runs">
		{#snippet aside()}
			<a class="hover:text-foreground underline" href="/runs">all {view.totalCohorts}</a>
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
