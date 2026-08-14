<script lang="ts">
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import FilterBar from '#lib/components/FilterBar.svelte';
	import FilterPills from '#lib/components/FilterPills.svelte';
	import FilterForm from '#lib/components/FilterForm.svelte';
	import WarningNotice from '#lib/components/WarningNotice.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import RunList from '#lib/components/RunList.svelte';
	import RunsTabs from '#lib/components/RunsTabs.svelte';
	import Note from '#lib/components/Note.svelte';
	import { Input } from '#lib/components/ui/input/index.js';
	import { RunsPageState } from '../home.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunsPageState(() => data, '/runs');
	// Seeded from `?q=` so a no-JS search round-trip leaves the box populated.
	view.query = view.serverQuery;
</script>

<svelte:head>
	<title>Runs / drizzle-rs benchmarks</title>
</svelte:head>

<Page>
	<PageHeader title="Runs">
		{#snippet subtitle()}{view.runsMeta}{/snippet}
		{#snippet aside()}
			<a
				class="underline underline-offset-2"
				href="/api/v1/runs/latest?suite={view.suite ?? view.suites[0] ?? ''}"
			>
				Latest run as JSON
			</a>
		{/snippet}
	</PageHeader>

	<RunsTabs />

	{#if view.warnings.length > 0}
		<div class="mt-7"><WarningNotice warnings={view.warnings} /></div>
	{/if}

	<FilterBar>
		{#if view.showSuiteFilter}
			<FilterPills label="suite" options={view.suiteFilters} />
		{/if}
		<FilterPills label="status" options={view.statusFilters} />
		{#snippet summary()}
			<FilterForm action="/runs" submitLabel="search" class="flex items-center gap-2">
				{#if view.suite}<input type="hidden" name="suite" value={view.suite} />{/if}
				{#if view.status}<input type="hidden" name="status" value={view.status} />{/if}
				<label for="run-search" class="sr-only">Search runs</label>
				<Input
					id="run-search"
					name="q"
					type="search"
					placeholder="Search runs, commits, targets"
					value={view.query}
					oninput={view.search}
					class="text-meta h-8 w-full font-mono sm:w-64"
				/>
			</FilterForm>
		{/snippet}
	</FilterBar>

	{#if !view.hasData}
		<div class="mt-7">
			<EmptyState title="No benchmark data has been published yet.">
				Publish a run to the <code class="font-mono">BENCH_DATA</code> bucket, or point
				<code class="font-mono">BENCH_DATA_DIR</code> at a local export.
			</EmptyState>
		</div>
	{:else}
		<RunList cohorts={view.filteredCohorts}>
			{#snippet empty()}
				<EmptyState title="No runs match these filters.">
					Clear the suite or status filter, or shorten the search.
				</EmptyState>
			{/snippet}
		</RunList>

		{#if view.filteredCohorts.length > 0}
			<div class="mt-4">
				<Note>
					A job listed more than once is the same work
					<a class="underline underline-offset-2" href="/runs/machines"
						>run again on another machine</a
					>.
				</Note>
			</div>
		{/if}
	{/if}
</Page>
