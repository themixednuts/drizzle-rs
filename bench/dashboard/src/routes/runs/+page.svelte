<script lang="ts">
	import {
		fmtDate,
		fmtDuration,
		runDisplayName,
		runStamp,
		shortHash,
		suiteLabel,
	} from '$lib/format';
	import Page from '$lib/components/Page.svelte';
	import PageHeader from '$lib/components/PageHeader.svelte';
	import Section from '$lib/components/Section.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import FilterPills from '$lib/components/FilterPills.svelte';
	import WarningNotice from '$lib/components/WarningNotice.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import DataTable from '$lib/components/data/DataTable.svelte';
	import Th from '$lib/components/data/Th.svelte';
	import Td from '$lib/components/data/Td.svelte';
	import Tr from '$lib/components/data/Tr.svelte';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { RunsPageState } from '../home.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunsPageState(() => data, '/runs');
</script>

<svelte:head>
	<title>runs - drizzle-rs/bench</title>
</svelte:head>

<Page>
	<PageHeader eyebrow="/ runs" title="Benchmark sets">
		{#snippet subtitle()}
			{view.totalCohorts} sets / {view.totalRuns} shards / {view.totalResults} results / {view.totalTargets}
			target ids
		{/snippet}
		{#snippet aside()}
			<a
				class="text-link hover:underline"
				href="/api/v1/runs/latest?suite={view.suite ?? view.suites[0] ?? ''}"
			>
				latest json
			</a>
		{/snippet}
	</PageHeader>

	<WarningNotice warnings={view.warnings} />

	<FilterBar>
		<FilterPills label="suite" options={view.suiteFilters} />
		<FilterPills label="status" options={view.statusFilters} />
		{#snippet summary()}
			<label for="run-search" class="sr-only">filter benchmark sets</label>
			<Input
				id="run-search"
				type="search"
				placeholder="run, commit, target..."
				value={view.query}
				oninput={view.search}
				class="text-meta h-6 w-full font-mono sm:w-56"
			/>
		{/snippet}
	</FilterBar>

	<Section title="{view.filteredCohorts.length} matching sets">
		{#if !view.hasData}
			<EmptyState title="No benchmark data has been published yet.">
				Publish a run to the <code class="font-mono">BENCH_DATA</code> bucket, or point
				<code class="font-mono">BENCH_DATA_DIR</code> at a local export.
			</EmptyState>
		{:else if view.filteredCohorts.length === 0}
			<EmptyState title="No benchmark sets match the current filters.">
				Clear the suite or status filter, or widen the search text.
			</EmptyState>
		{:else}
			<DataTable>
				<Table.Header>
					<Table.Row class="border-0">
						<Th>set</Th>
						<Th>suite</Th>
						<Th>status</Th>
						<Th>class</Th>
						<Th>commit</Th>
						<Th numeric>targets</Th>
						<Th numeric>results</Th>
						<Th>shard runs</Th>
						<Th numeric>duration</Th>
						<Th numeric>started</Th>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each view.filteredCohorts as run (run.id)}
						<Tr>
							<Td>
								<a class="text-link hover:underline" href="/compare?cohort={run.id}">
									{runDisplayName(run)}
								</a>
								<div class="text-micro text-muted-foreground">{run.id}</div>
							</Td>
							<Td tone="muted">{suiteLabel(run.suite)}</Td>
							<Td><StatusBadge status={run.status} /></Td>
							<Td tone="muted">{run.class}</Td>
							<Td>{shortHash(run.git)}</Td>
							<Td numeric>{run.targets.length}</Td>
							<Td numeric>{run.result_count}</Td>
							<Td wrap>
								<span class="flex flex-wrap gap-x-1.5 gap-y-0.5">
									{#each run.run_ids as runId (runId)}
										<a class="text-link hover:underline" href="/runs/{runId}">{runStamp(runId)}</a>
									{/each}
								</span>
							</Td>
							<Td numeric tone="muted">{fmtDuration(run.start, run.end)}</Td>
							<Td numeric tone="muted">{fmtDate(run.start)}</Td>
						</Tr>
					{/each}
				</Table.Body>
			</DataTable>
		{/if}
	</Section>
</Page>
