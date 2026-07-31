<script lang="ts">
	import { loadTimeseries } from '#lib/api.remote';
	import LatencyBars from '#lib/components/LatencyBars.svelte';
	import QueryMetricBars from '#lib/components/QueryMetricBars.svelte';
	import SparkLine from '#lib/components/SparkLine.svelte';
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import MetricGrid from '#lib/components/MetricGrid.svelte';
	import MetricTile from '#lib/components/MetricTile.svelte';
	import Note from '#lib/components/Note.svelte';
	import Hint from '#lib/components/Hint.svelte';
	import StatusBadge from '#lib/components/StatusBadge.svelte';
	import TargetLabel from '#lib/components/TargetLabel.svelte';
	import DataTable from '#lib/components/data/DataTable.svelte';
	import Th from '#lib/components/data/Th.svelte';
	import Td from '#lib/components/data/Td.svelte';
	import Tr from '#lib/components/data/Tr.svelte';
	import * as Table from '#lib/components/ui/table/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import * as Accordion from '#lib/components/ui/accordion/index.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import {
		fmtCpu,
		fmtDate,
		fmtDuration,
		fmtGb,
		fmtLatency,
		fmtPct,
		fmtRps,
		shortHash,
		suiteLabel,
	} from '#lib/format';
	import { RunDetailState } from './run-detail.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunDetailState(() => data);

	const manifest = $derived(view.manifest);
	const runner = $derived(manifest.runner);
</script>

<svelte:head>
	<title>{view.runName} - drizzle-rs/bench</title>
</svelte:head>

<Page>
	<PageHeader eyebrow="/ runs / detail" title={view.runName}>
		{#snippet subtitle()}
			{manifest.run_id}{manifest.cohort_id ? ` / cohort ${manifest.cohort_id}` : ''} / {suiteLabel(
				manifest.suite,
			)} / {shortHash(manifest.git)} / {fmtDate(manifest.start)}
		{/snippet}
		{#snippet aside()}
			<span class="flex items-center gap-2">
				<StatusBadge status={manifest.status} />
				<a class="text-link hover:underline" href="/runs">all runs</a>
			</span>
		{/snippet}
	</PageHeader>

	{#if view.kpiTarget}
		{@const target = view.kpiTarget}
		<div
			class="text-caption text-muted-foreground flex flex-wrap items-baseline gap-x-3 gap-y-1 pt-6 font-mono uppercase"
		>
			<span>headline numbers</span>
			<span class="normal-case {target.isOurs ? 'text-foreground tracking-normal' : ''}">
				{target.label}
			</span>
			<span class="ml-auto normal-case">{runner.os} / {runner.class}</span>
		</div>
		<MetricGrid>
			{#each view.kpis as item (item.label)}
				<MetricTile label={item.label} value={item.value} detail={item.detail} hint={item.hint} />
			{/each}
		</MetricGrid>
	{/if}

	<Section title="run metadata">
		<DataTable>
			<Table.Body>
				<Tr>
					<Td tone="muted" class="w-36">suite</Td>
					<Td>{suiteLabel(manifest.suite)}</Td>
					<Td tone="muted" class="w-36">workload</Td>
					<Td>{manifest.workload}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">commit</Td>
					<Td>{manifest.git}</Td>
					<Td tone="muted">duration</Td>
					<Td>{fmtDuration(manifest.start, manifest.end)}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">runner</Td>
					<Td>{runner.class} / {runner.os}</Td>
					<Td tone="muted">hardware</Td>
					<Td>{runner.cpu} / {runner.cores}c / {fmtGb(runner.mem_gb)}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">trials</Td>
					<Td>{manifest.trials.count} trials, {manifest.trials.aggregate} across trials</Td>
					<Td tone="muted">seed</Td>
					<Td>{manifest.seed}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">metrics</Td>
					<Td>
						{runner.metrics
							? `${runner.metrics.cpu_scope} cpu / ${runner.metrics.memory_scope} memory`
							: 'scope not declared'}
					</Td>
					<Td tone="muted">network</Td>
					<Td>{runner.metrics?.network_scope ?? 'scope not declared'}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">
						<Hint
							hint="highest single-core utilization seen on the runner; this is load, not spare capacity"
						>
							peak core cpu
						</Hint>
					</Td>
					<Td>
						{fmtCpu(runner.headroom.cpu_peak)}
						{#if runner.headroom.cpu_mean_peak != null}
							<span class="text-muted-foreground">
								/ mean-core peak {fmtCpu(runner.headroom.cpu_mean_peak)}
							</span>
						{/if}
						<span class="text-muted-foreground">
							/ net {runner.headroom.net_peak == null
								? 'unmeasured'
								: fmtCpu(runner.headroom.net_peak)}
						</span>
					</Td>
					<Td tone="muted">targets</Td>
					<Td>{manifest.targets.length}</Td>
				</Tr>
			</Table.Body>
		</DataTable>

		<div class="mt-4">
			<Note>
				The load generator, the target server and its database all run on this one runner and share
				its cores, so target CPU and load-generator CPU come out of the same budget. Numbers here
				are only comparable to other targets in this same run. See
				<a class="text-link underline" href="/methodology">methodology</a>.
			</Note>
		</div>
	</Section>

	<Section title="load and dataset">
		<DataTable>
			<Table.Body>
				<Tr>
					<Td tone="muted" class="w-36">executor</Td>
					<Td>{manifest.load.executor}</Td>
					<Td tone="muted" class="w-36">stages</Td>
					<Td>{manifest.load.stages}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">load duration</Td>
					<Td>{manifest.load.duration_s}s</Td>
					<Td tone="muted">max vus</Td>
					<Td>{manifest.load.max_vus.toLocaleString()}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">requests</Td>
					<Td>{manifest.load.requests.toLocaleString()}</Td>
					<Td tone="muted">pacing</Td>
					<Td>{manifest.load.pacing ?? 'not declared'}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">orders</Td>
					<Td>{manifest.dataset.orders.toLocaleString()}</Td>
					<Td tone="muted">customers</Td>
					<Td>{manifest.dataset.customers.toLocaleString()}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">products</Td>
					<Td>{manifest.dataset.products.toLocaleString()}</Td>
					<Td tone="muted">suppliers</Td>
					<Td>{manifest.dataset.suppliers.toLocaleString()}</Td>
				</Tr>
				<Tr>
					<Td tone="muted">details/order</Td>
					<Td>{manifest.dataset.details_per_order}</Td>
					<Td tone="muted">seed</Td>
					<Td>{manifest.seed}</Td>
				</Tr>
			</Table.Body>
		</DataTable>
	</Section>

	{#if view.queries.length > 0}
		<Section title="request mix">
			{#snippet aside()}{view.totalQueryMix.toLocaleString()} materialized requests{/snippet}

			<div class="grid gap-6 md:grid-cols-[minmax(0,0.7fr)_minmax(0,1.3fr)]">
				<Note variant="warn">
					This is workload composition only: the generated HTTP routes and how often each route
					appears in the request list. Metric graphs are per target below, where route-level RPS and
					latency come from measured request samples.
				</Note>

				<ul class="text-caption grid gap-2 font-mono tracking-normal">
					{#each view.queries as query (query.id)}
						<li class="flex flex-wrap items-baseline justify-between gap-x-3">
							<span class="min-w-0">
								<span class="text-foreground">{query.name}</span>
								<span class="text-muted-foreground ml-1.5">{query.method} {query.path}</span>
							</span>
							<span class="text-muted-foreground tabular-nums">
								{fmtPct(view.queryShare(query))} / {query.mix.toLocaleString()}
							</span>
						</li>
					{/each}
				</ul>
			</div>
		</Section>

		<Section title="query catalog">
			{#snippet aside()}{view.queries.length} operations / SQL shapes{/snippet}

			<Accordion.Root type="single" class="w-full">
				{#each view.queries as query (query.id)}
					<Accordion.Item value={query.id}>
						<Accordion.Trigger class="text-meta font-mono">
							<span class="flex min-w-0 flex-1 flex-wrap items-baseline justify-between gap-x-3">
								<span>{query.name}</span>
								<span class="text-muted-foreground">
									{query.method}
									{query.path} / {query.mix.toLocaleString()}
								</span>
							</span>
						</Accordion.Trigger>
						<Accordion.Content>
							<dl class="text-meta grid grid-cols-[5.5rem_minmax(0,1fr)] gap-x-3 font-mono">
								<dt class="text-muted-foreground">params</dt>
								<dd class="break-words">
									{query.params.length ? query.params.join(', ') : 'none'}
								</dd>
							</dl>
							{#each query.sql as shape (shape.dialect + shape.text)}
								<pre
									class="border-border bg-muted text-caption text-foreground-secondary mt-2 overflow-x-auto border px-3 py-2 font-mono tracking-normal"><code
										>{shape.text}</code
									></pre>
							{/each}
						</Accordion.Content>
					</Accordion.Item>
				{/each}
			</Accordion.Root>
		</Section>
	{/if}

	<Section title="target summary">
		<DataTable>
			<Table.Header>
				<Table.Row class="border-0">
					<Th>target</Th>
					<Th>group</Th>
					<Th numeric hint="median requests/second across trials">rps median</Th>
					<Th numeric>peak</Th>
					<Th numeric hint="median across trials of each trial's mean latency">lat mean</Th>
					<Th numeric>lat p95</Th>
					<Th numeric>lat p99</Th>
					<Th numeric hint="median across trials of mean-across-cores utilization">cpu median</Th>
					<Th numeric>err</Th>
					<Th class="w-40">throughput</Th>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each view.sortedSummaries as summary (summary.target_id)}
					{@const p = summary.primary}
					<Tr baseline={view.isBaseline(summary)}>
						<Td wrap>
							<TargetLabel
								display={view.targetDisplay(summary.target_id)}
								targetId={summary.target_id}
							/>
						</Td>
						<Td tone="muted">{view.targetGroup(summary)}</Td>
						<Td numeric>{fmtRps(p.rps.avg)}</Td>
						<Td numeric tone="secondary">{fmtRps(p.rps.peak)}</Td>
						<Td numeric tone="secondary">{fmtLatency(p.latency.avg)}</Td>
						<Td numeric>{fmtLatency(p.latency.p95)}</Td>
						<Td numeric tone="secondary">{fmtLatency(p.latency.p99)}</Td>
						<Td numeric tone="muted">{fmtCpu(p.cpu.avg)}</Td>
						<Td numeric tone="muted">{fmtPct(p.err)}</Td>
						<Td>
							<div class="bg-muted h-1.5 w-full" role="img" aria-label="{fmtRps(p.rps.avg)} rps">
								<div
									class="h-full {view.isBaseline(summary) ? 'bg-primary' : 'bg-foreground-faint'}"
									style="width: {view.barWidth(summary)}"
								></div>
							</div>
						</Td>
					</Tr>
				{/each}
			</Table.Body>
		</DataTable>
	</Section>

	{#each view.groups as [groupName, groupItems] (groupName)}
		<Section title="{groupName} detail">
			{#snippet aside()}
				{groupItems.length} target{groupItems.length === 1 ? '' : 's'}
			{/snippet}

			<div class="grid gap-5">
				{#each groupItems as summary (summary.target_id)}
					{@const p = summary.primary}
					{@const meta = view.targetMeta(summary.target_id)}
					{@const display = view.targetDisplay(summary.target_id)}
					{@const metric = view.metricFor(summary.target_id)}
					<article class="border-border-soft border-b pb-5 last:border-b-0">
						<div class="mb-2 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
							<h3 class="text-lead flex flex-wrap items-baseline gap-x-2 font-medium">
								{display.name}
								<TargetLabel {display} targetId={summary.target_id} />
							</h3>
							<Badge
								variant="outline"
								class="text-micro font-mono uppercase {p.err > 0
									? 'text-negative'
									: 'text-positive'}"
							>
								{fmtPct(p.err)} err
							</Badge>
						</div>

						{#if view.targetDescription(summary.target_id)}
							<p class="measure text-meta text-muted-foreground mb-3">
								{view.targetDescription(summary.target_id)}
							</p>
						{/if}
						{#if display.sqlVariant}
							<p class="measure text-meta text-muted-foreground mb-3">
								sql variant: {display.sqlVariant}
							</p>
						{/if}
						{#if display.incomplete}
							<p class="measure text-meta text-muted-foreground mb-3">
								this run's manifest carries no target metadata for
								<code class="font-mono">{summary.target_id}</code>; details below are unavailable.
							</p>
						{/if}

						<dl
							class="mb-4 grid grid-cols-2 gap-3 font-mono sm:grid-cols-4"
							aria-label="headline metrics for {display.name}"
						>
							{#each [{ term: 'rps', value: fmtRps(p.rps.avg) }, { term: 'p95', value: fmtLatency(p.latency.p95) }, { term: 'p99', value: fmtLatency(p.latency.p99) }, { term: 'cpu', value: fmtCpu(p.cpu.avg) }] as stat (stat.term)}
								<div>
									<dt class="text-caption text-muted-foreground uppercase">{stat.term}</dt>
									<dd class="text-lead mt-0.5 font-medium tabular-nums">{stat.value}</dd>
								</div>
							{/each}
						</dl>

						<div class="mb-3 grid gap-6 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
							<div>
								<div class="text-caption text-muted-foreground mb-1 font-mono uppercase">
									latency distribution
								</div>
								<LatencyBars latency={p.latency} />
							</div>

							<div>
								<Tabs.Root
									value={metric}
									onValueChange={(next) => view.selectMetric(summary.target_id, next)}
								>
									<Tabs.List variant="line" aria-label="metric for {display.name}">
										{#each view.metricTabs(summary) as tab (tab.key)}
											<Tabs.Trigger value={tab.key} class="font-mono">{tab.label}</Tabs.Trigger>
										{/each}
									</Tabs.List>
								</Tabs.Root>

								<p class="text-micro text-muted-foreground mt-1.5 font-mono tracking-normal">
									{view.metricHelp(summary.target_id)}
								</p>

								<svelte:boundary>
									{#snippet pending()}
										<Skeleton class="mt-2 h-12 w-full" />
									{/snippet}

									{#snippet failed(error, reset)}
										<div
											class="border-negative text-caption mt-2 grid justify-items-start gap-1.5 border-l-2 py-2 pl-3 font-mono tracking-normal"
										>
											<p>Timeseries for this target could not be loaded.</p>
											<p class="text-muted-foreground">
												{error instanceof Error ? error.message : String(error)}
											</p>
											<Button variant="outline" size="xs" onclick={reset}>retry</Button>
										</div>
									{/snippet}

									{@const ts = await loadTimeseries({
										runId: manifest.run_id,
										targetId: summary.target_id,
									})}
									{#if ts}
										<SparkLine points={ts.points} {metric} trialCount={manifest.trials.count} />
										<QueryMetricBars
											queries={view.queries}
											points={ts.points}
											{metric}
											trialCount={manifest.trials.count}
										/>
									{:else}
										<div
											class="text-caption text-muted-foreground flex h-12 items-center font-mono"
										>
											no timeseries data
										</div>
									{/if}
								</svelte:boundary>
							</div>
						</div>

						<Separator class="mb-3" />

						<DataTable>
							<Table.Body>
								{#if !display.incomplete}
									<Tr>
										<Td tone="muted" class="w-40">runtime</Td>
										<Td>{meta.runtime.name} {meta.runtime.ver}</Td>
										<Td tone="muted" class="w-40">orm</Td>
										<Td>{meta.orm.name} {meta.orm.ver}</Td>
									</Tr>
									<Tr>
										<Td tone="muted">driver</Td>
										<Td>{meta.driver.name} {meta.driver.ver}</Td>
										<Td tone="muted">wire</Td>
										<Td>{meta.wire.format}</Td>
									</Tr>
									<Tr>
										<Td tone="muted">workers / pool</Td>
										<Td>{meta.proc.workers} / {meta.pool.max}</Td>
										<Td tone="muted">fair contract</Td>
										<Td>{meta.fair.contract} / {meta.contract.ver}</Td>
									</Tr>
									<Tr>
										<Td tone="muted">prepared statements</Td>
										<Td>
											{meta.db.prepared == null ? 'not declared' : meta.db.prepared ? 'yes' : 'no'}
										</Td>
										<Td tone="muted">data access</Td>
										<Td>{meta.data_access ?? 'not declared'}</Td>
									</Tr>
								{/if}
								<Tr>
									<Td tone="muted">spread rps</Td>
									<Td>{fmtRps(summary.spread.rps.min)} - {fmtRps(summary.spread.rps.max)}</Td>
									<Td tone="muted">spread p95</Td>
									<Td>
										{fmtLatency(summary.spread.p95.min)} - {fmtLatency(summary.spread.p95.max)}
									</Td>
								</Tr>
								<Tr>
									<Td tone="muted">saturation rps</Td>
									<Td>{fmtRps(summary.saturation.knee_rps)}</Td>
									<Td tone="muted">saturation p95</Td>
									<Td>{fmtLatency(summary.saturation.knee_p95)}</Td>
								</Tr>
							</Table.Body>
						</DataTable>
					</article>
				{/each}
			</div>
		</Section>
	{/each}
</Page>
