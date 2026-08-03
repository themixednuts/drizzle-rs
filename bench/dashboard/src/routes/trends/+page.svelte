<script lang="ts">
	import TrendChart from '#lib/components/TrendChart.svelte';
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import FilterBar from '#lib/components/FilterBar.svelte';
	import FilterPills from '#lib/components/FilterPills.svelte';
	import FilterForm from '#lib/components/FilterForm.svelte';
	import SelectField from '#lib/components/SelectField.svelte';
	import WarningNotice from '#lib/components/WarningNotice.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import MetricGrid from '#lib/components/MetricGrid.svelte';
	import MetricTile from '#lib/components/MetricTile.svelte';
	import Note from '#lib/components/Note.svelte';
	import DataTable from '#lib/components/data/DataTable.svelte';
	import Th from '#lib/components/data/Th.svelte';
	import Td from '#lib/components/data/Td.svelte';
	import Tr from '#lib/components/data/Tr.svelte';
	import * as Table from '#lib/components/ui/table/index.js';
	import { fmtCpu, fmtLatency, fmtPct, fmtRps, runStamp, shortHash } from '#lib/format';
	import { TrendsPageState } from './trends.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new TrendsPageState(() => data);

	/** Which trend fields get a chart, and which metric hue each one inherits. */
	const CHARTS = [
		{ field: 'rps_avg', metric: 'rps', label: 'rps median across trials' },
		{ field: 'latency_p95', metric: 'latency', label: 'p95 latency' },
		{ field: 'latency_p99', metric: 'latency', label: 'p99 latency' },
		{ field: 'cpu_avg', metric: 'cpu', label: 'cpu median across trials' },
	] as const;
</script>

<svelte:head>
	<title>trends - drizzle-rs/bench</title>
</svelte:head>

<Page>
	<PageHeader title="Trends">
		{#snippet subtitle()}
			{view.targetLabel ?? 'select a library'} &#183; {view.trends.length} run{view.trends
				.length === 1
				? ''
				: 's'}
		{/snippet}
		{#snippet aside()}
			{#if view.latest}
				latest
				<a class="text-link hover:underline" href="/runs/{view.latest.run_id}">
					{shortHash(view.latest.git)}
				</a>
			{/if}
		{/snippet}
	</PageHeader>

	<WarningNotice warnings={view.warnings} />

	<FilterBar>
		{#if view.showSuiteFilter}
			<FilterPills label="suite" options={view.suiteFilters} />
		{/if}
		<FilterForm action="/trends" class="contents">
			{#if view.suite}
				<input type="hidden" name="suite" value={view.suite} />
			{/if}
			<SelectField
				id="trend-target"
				name="target"
				label="target"
				value={view.targetKey ?? ''}
				options={view.targetOptions}
				placeholder="select target..."
				class="min-w-0 flex-1"
			/>
		</FilterForm>
	</FilterBar>

	{#if !view.targetKey}
		<div class="mt-7">
			<EmptyState title="Select a target to view its history.">
				The dropdown lists the targets in the newest benchmark set; history is then fetched for that
				target alone.
			</EmptyState>
		</div>
	{:else if view.trends.length === 0}
		<div class="mt-7">
			<EmptyState title="No successful trend data for {view.targetLabel}.">
				Runs with this target will appear here after CI publishes their summaries.
			</EmptyState>
		</div>
	{:else}
		<div class="mt-7">
			<Note>
				Every point is one benchmark set for this one target on this one runner OS. An up arrow
				means the set improved on the one before it — for latency, CPU and errors that means the
				number fell.
			</Note>
		</div>

		<MetricGrid>
			{#each view.kpis as kpi (kpi.label)}
				<MetricTile
					label={kpi.label}
					value={kpi.value}
					hint={kpi.hint}
					detail={kpi.detail}
					delta={kpi.delta}
				/>
			{/each}
		</MetricGrid>

		<Section title="charts">
			{#each CHARTS as chart (chart.field)}
				<TrendChart
					points={view.trends}
					field={chart.field}
					metric={chart.metric}
					label={chart.label}
				/>
			{/each}
			{#if view.latest?.mem_avg !== undefined}
				<TrendChart
					points={view.trends}
					field="mem_avg"
					metric="mem"
					label="memory median across trials"
				/>
			{/if}
		</Section>

		<Section title="run history">
			<DataTable>
				<Table.Header>
					<Table.Row class="border-0">
						<Th>run</Th>
						<Th>commit</Th>
						<Th numeric hint="median requests/second across trials, at the paced suite's fixed load"
							>rps median</Th
						>
						<Th
							numeric
							hint="highest single sample bucket within the paced run; not a capacity figure"
							>busiest second</Th
						>
						<Th numeric>lat p95</Th>
						<Th numeric>lat p99</Th>
						<Th numeric hint="median across trials of mean-across-cores utilization">cpu median</Th>
						<Th numeric>err</Th>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each view.reversedTrends as point (point.run_id)}
						<Tr>
							<Td>
								<a class="text-link hover:underline" href="/runs/{point.run_id}">
									{runStamp(point.run_id)}
								</a>
							</Td>
							<Td tone="muted">{shortHash(point.git)}</Td>
							<Td numeric>{fmtRps(point.rps_avg)}</Td>
							<Td numeric tone="secondary">{fmtRps(point.rps_peak)}</Td>
							<Td numeric>{fmtLatency(point.latency_p95)}</Td>
							<Td numeric tone="secondary">{fmtLatency(point.latency_p99)}</Td>
							<Td numeric tone="muted">{fmtCpu(point.cpu_avg)}</Td>
							<Td numeric tone="muted">{fmtPct(point.err)}</Td>
						</Tr>
					{/each}
				</Table.Body>
			</DataTable>
		</Section>
	{/if}
</Page>
