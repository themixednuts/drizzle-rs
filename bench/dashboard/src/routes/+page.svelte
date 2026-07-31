<script lang="ts">
	import BoxWhisker from '$lib/components/BoxWhisker.svelte';
	import Page from '$lib/components/Page.svelte';
	import PageHeader from '$lib/components/PageHeader.svelte';
	import Section from '$lib/components/Section.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import FilterPills from '$lib/components/FilterPills.svelte';
	import WarningNotice from '$lib/components/WarningNotice.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import MetricGrid from '$lib/components/MetricGrid.svelte';
	import MetricTile from '$lib/components/MetricTile.svelte';
	import Note from '$lib/components/Note.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import TargetLabel from '$lib/components/TargetLabel.svelte';
	import Delta from '$lib/components/Delta.svelte';
	import DataTable from '$lib/components/data/DataTable.svelte';
	import Th from '$lib/components/data/Th.svelte';
	import Td from '$lib/components/data/Td.svelte';
	import Tr from '$lib/components/data/Tr.svelte';
	import * as Table from '$lib/components/ui/table/index.js';
	import {
		fmtCpu,
		fmtDate,
		fmtDuration,
		fmtLatency,
		fmtPct,
		fmtRps,
		runDisplayName,
		shardLabel,
		shortHash,
		suiteLabel,
	} from '$lib/format';
	import { RunsPageState } from './home.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunsPageState(() => data);
</script>

<svelte:head>
	<title>drizzle-rs/bench</title>
</svelte:head>

<Page>
	<PageHeader eyebrow="/ overview" title="drizzle-rs/bench">
		{#snippet subtitle()}{view.overviewMeta}{/snippet}
		{#snippet aside()}
			{#if view.latest}
				latest set
				<a
					class="text-link hover:underline"
					href="/runs/{view.latest.cohort.representative_run_id}"
				>
					{shortHash(view.latest.cohort.git)}
				</a>
				/ {fmtDate(view.latest.cohort.start)}
			{/if}
		{/snippet}
	</PageHeader>

	<WarningNotice warnings={view.warnings} />

	{#if !view.hasData}
		<div class="pt-8">
			<EmptyState title="No benchmark data has been published yet.">
				Publish a run to the <code class="font-mono">BENCH_DATA</code> bucket, or point
				<code class="font-mono">BENCH_DATA_DIR</code> at a local
				<code class="font-mono">dashboard-bench-data</code> export and reload.
			</EmptyState>
		</div>
	{/if}

	{#if view.kpiTarget}
		{@const target = view.kpiTarget}
		<div
			class="text-caption text-muted-foreground flex flex-wrap items-baseline gap-x-3 gap-y-1 pt-6 font-mono uppercase"
		>
			<span>headline numbers</span>
			<span class="normal-case {target.isOurs ? 'text-foreground tracking-normal' : ''}">
				{target.label}
			</span>
			<span class="ml-auto normal-case">
				{shardLabel(target.summary.runner_os, target.summary.run_id)}
			</span>
		</div>
		<MetricGrid>
			{#each view.kpis as item (item.label)}
				<MetricTile label={item.label} value={item.value} detail={item.detail} hint={item.hint} />
			{/each}
		</MetricGrid>
	{/if}

	<FilterBar>
		<FilterPills label="suite" options={view.suiteFilters} />
		<FilterPills label="status" options={view.statusFilters} />
		{#snippet summary()}{view.filterMeta}{/snippet}
	</FilterBar>

	<Section title="latest leaderboard">
		{#snippet aside()}
			{#if view.latest}
				<a class="text-link hover:underline" href="/compare?cohort={view.latest.cohort.id}">
					compare all
				</a>
			{/if}
		{/snippet}

		{#if view.sections.length === 0}
			<EmptyState title="No successful run summaries are available.">
				A set appears here once at least one of its shards publishes target summaries.
			</EmptyState>
		{:else}
			<Note>
				Rows are ranked only against other targets in the same database family. Within a set,
				different families are produced by different CI jobs on different VMs, so numbers are
				comparable down a section and not across sections — see
				<a class="text-link underline" href="/methodology">methodology</a>.
			</Note>

			{#each view.sections as section (section.key)}
				<div class="mt-7">
					<div class="mb-1.5 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
						<h3 class="text-lead font-medium">{section.label}</h3>
						<span class="text-micro text-muted-foreground font-mono tracking-normal">
							{section.rows.length} result{section.rows.length === 1 ? '' : 's'}
							{#if section.shards.length > 0}
								/ {section.shards.map((shard) => shardLabel(shard.os, shard.run_id)).join(' · ')}
							{/if}
						</span>
					</div>
					{#if section.note}
						<div class="mb-3">
							<Note>{section.note}</Note>
						</div>
					{/if}

					<DataTable>
						<Table.Header>
							<Table.Row class="border-0">
								<Th numeric class="w-8">#</Th>
								<Th>target</Th>
								<Th>vm</Th>
								<Th numeric hint="median requests/second across trials">rps median</Th>
								<Th numeric hint="median across trials of each trial's mean latency">lat mean</Th>
								<Th numeric hint="median across trials of the 95th percentile">lat p95</Th>
								<Th numeric hint="median across trials of the 99th percentile">lat p99</Th>
								<Th numeric hint="median across trials of mean-across-cores utilization">
									cpu median
								</Th>
								<Th numeric>err</Th>
								<Th class="w-56">throughput spread</Th>
								<Th numeric hint="positive means drizzle is ahead of this target">drizzle Δ</Th>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each section.rows as row (row.id)}
								{@const summary = row.summary}
								{@const p = summary.primary}
								<Tr
									baseline={row.isBaseline}
									emphasis={view.rowEmphasis(summary)}
									onpointerenter={() => view.hoverTarget(summary)}
									onpointerleave={view.clearHover}
								>
									<Td numeric tone="muted">
										{row.rank === null ? '-' : String(row.rank).padStart(2, '0')}
									</Td>
									<Td wrap>
										<TargetLabel
											display={view.targetDisplay(summary)}
											href="/runs/{summary.run_id}"
											targetId={summary.target_id}
										/>
										{#if view.targetDisplay(summary).sqlVariant}
											<div class="text-micro text-muted-foreground mt-0.5">
												sql variant: {view.targetDisplay(summary).sqlVariant}
											</div>
										{/if}
									</Td>
									<Td tone="muted">
										<a class="hover:text-link hover:underline" href="/runs/{summary.run_id}">
											{shardLabel(summary.runner_os, summary.run_id)}
										</a>
									</Td>
									<Td numeric>{fmtRps(p.rps.avg)}</Td>
									<Td numeric tone="secondary">{fmtLatency(p.latency.avg)}</Td>
									<Td numeric>{fmtLatency(p.latency.p95)}</Td>
									<Td numeric tone="secondary">{fmtLatency(p.latency.p99)}</Td>
									<Td numeric tone="muted">{fmtCpu(p.cpu.avg)}</Td>
									<Td numeric tone="muted">{fmtPct(p.err)}</Td>
									<Td>
										<BoxWhisker
											box={view.throughputBox(summary)}
											extent={section.extent}
											label={view.throughputLabel(summary)}
											summaryLabel={view.throughputSummaryLabel(summary)}
											accent={row.isBaseline}
										/>
									</Td>
									<Td numeric>
										<Delta
											text={row.deltaText}
											direction={row.deltaDirection}
											hint={row.deltaTitle}
										/>
									</Td>
								</Tr>
							{/each}
						</Table.Body>
					</DataTable>
				</div>
			{/each}
		{/if}
	</Section>

	<Section title="recent benchmark sets">
		{#snippet aside()}
			<a class="text-link hover:underline" href="/runs">all {view.totalCohorts}</a>
		{/snippet}

		{#if view.cohorts.length === 0}
			<EmptyState title="No runs match the selected filters.">
				Try a different suite or status.
			</EmptyState>
		{:else}
			<DataTable>
				<Table.Header>
					<Table.Row class="border-0">
						<Th>set</Th>
						<Th>suite</Th>
						<Th>status</Th>
						<Th numeric>targets</Th>
						<Th numeric>results</Th>
						<Th>shards</Th>
						<Th>commit</Th>
						<Th numeric>duration</Th>
						<Th numeric>started</Th>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each view.recentCohorts as run (run.id)}
						<Tr>
							<Td>
								<a class="text-link hover:underline" href="/compare?cohort={run.id}">
									{runDisplayName(run)}
								</a>
								<div class="text-micro text-muted-foreground">{run.id}</div>
							</Td>
							<Td tone="muted">{suiteLabel(run.suite)}</Td>
							<Td><StatusBadge status={run.status} /></Td>
							<Td numeric>{run.targets.length}</Td>
							<Td numeric>{run.result_count}</Td>
							<Td wrap>
								<span class="flex flex-wrap gap-x-1.5 gap-y-0.5">
									{#each run.run_ids as runId (runId)}
										<a class="text-link hover:underline" href="/runs/{runId}">
											{runId.split('_')[0]}
										</a>
									{/each}
								</span>
							</Td>
							<Td tone="muted">{shortHash(run.git)}</Td>
							<Td numeric tone="muted">{fmtDuration(run.start, run.end)}</Td>
							<Td numeric tone="muted">{fmtDate(run.start)}</Td>
						</Tr>
					{/each}
				</Table.Body>
			</DataTable>
		{/if}
	</Section>
</Page>
