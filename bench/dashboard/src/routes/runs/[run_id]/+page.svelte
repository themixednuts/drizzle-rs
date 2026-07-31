<script lang="ts">
	import LatencyBars from '#lib/components/LatencyBars.svelte';
	import OverlayChart from '#lib/components/OverlayChart.svelte';
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
	import DataTable from '#lib/components/data/DataTable.svelte';
	import Td from '#lib/components/data/Td.svelte';
	import Tr from '#lib/components/data/Tr.svelte';
	import * as Table from '#lib/components/ui/table/index.js';
	import { cn } from '#lib/utils.js';
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
	import { runFacts } from '#lib/run-facts';
	import { classLabel } from '#lib/run-name';
	import { RunDetailState } from './run-detail.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunDetailState(() => data);

	const manifest = $derived(view.manifest);
	const runner = $derived(manifest.runner);
</script>

<svelte:head>
	<title>{view.runName} / drizzle-rs benchmarks</title>
</svelte:head>

<Page>
	<PageHeader title={view.runName} back={{ href: '/runs', label: 'all runs' }}>
		{#snippet aside()}
			<StatusBadge status={manifest.status} />
		{/snippet}
	</PageHeader>

	<!--
		The comp's meta strip: the facts that identify this run, on one mono line under the title
		instead of scattered across a metadata table and a KPI header. Everything the strip does not
		carry is still in "About this run" below.
	-->
	<div
		class="border-border text-caption text-muted-foreground mt-4 flex flex-wrap gap-x-7 gap-y-2 border-b pb-5 font-mono"
	>
		<span>commit {shortHash(manifest.git)}</span>
		<span>{fmtDate(manifest.start)}</span>
		<span>{fmtDuration(manifest.start, manifest.end)}</span>
		<span>{manifest.trials.count} trials each</span>
		<span>{runner.cores}-core {runner.os}</span>
		{#if classLabel(runner.class)}<span>{classLabel(runner.class)}</span>{/if}
	</div>

	<Section>
		<OverlayChart chart={view.overlays.rps} height={190} />
	</Section>

	<Section>
		<OverlayChart chart={view.overlays.latency} height={150} />
	</Section>

	<!-- One section per target, in throughput order, with the accent edge on ours. -->
	{#each view.sortedSummaries as summary (summary.target_id)}
		{@const p = summary.primary}
		{@const meta = view.targetMeta(summary.target_id)}
		{@const display = view.targetDisplay(summary.target_id)}
		{@const ours = view.isBaseline(summary)}
		<Section class={cn(ours && 'border-l-primary border-l-[3px]')}>
			<div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
				<h2 class={cn('text-heading font-semibold', ours && 'text-link')}>{display.name}</h2>
				<span class="text-meta text-foreground-secondary">{display.note}</span>
				<span class="text-caption text-muted-foreground ml-auto font-mono">
					{view.rangeText(summary)}
				</span>
			</div>

			{#if view.targetDescription(summary.target_id)}
				<p class="measure text-meta text-muted-foreground mt-2">
					{view.targetDescription(summary.target_id)}
				</p>
			{/if}
			{#if display.incomplete}
				<p class="measure text-meta text-muted-foreground mt-2">
					this run's manifest carries no target metadata for
					<code class="font-mono">{summary.target_id}</code>; details below are unavailable.
				</p>
			{/if}

			<div class="mt-5">
				<MetricGrid>
					<MetricTile
						label="requests/sec"
						value={fmtRps(p.rps.avg)}
						detail="peak {fmtRps(p.rps.peak)}"
						hint="median requests/second across trials"
					/>
					<MetricTile
						label="typical latency"
						value={fmtLatency(p.latency.avg)}
						detail="median across trials"
						hint="median across trials of each trial's mean latency"
					/>
					<MetricTile
						label="slowest 5%"
						value={fmtLatency(p.latency.p95)}
						detail="p99 {fmtLatency(p.latency.p99)}"
						hint="p95: 95 of 100 requests finished faster than this, median across trials"
					/>
					<MetricTile
						label="cpu"
						value={fmtCpu(p.cpu.avg)}
						detail="peak core {fmtCpu(p.cpu.peak)}"
						hint="median across trials of mean-across-cores utilization; peak core is the highest single-core utilization"
					/>
					{#if p.mem}
						<MetricTile
							label="memory"
							value="{p.mem.avg.toFixed(1)}MB"
							detail="peak {p.mem.peak.toFixed(1)}MB"
							hint="median resident memory across trials"
						/>
					{/if}
					<MetricTile
						label="errors"
						value={fmtPct(p.err)}
						detail="of all requests"
						hint="errored requests / total requests; above 0.5% the throughput number is not comparable"
					/>
				</MetricGrid>
			</div>

			{#if view.variantNote(summary.target_id)}
				{@const variant = view.variantNote(summary.target_id)!}
				<p class="text-meta text-muted-foreground mt-4">
					<span class="text-foreground-secondary">SQL:</span>
					{#if variant.elided}
						<Hint hint={variant.full}>{variant.short}</Hint>
					{:else}
						{variant.short}
					{/if}
				</p>
			{/if}

			<div class="mt-6 grid gap-7 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
				<div>
					<div class="text-micro text-muted-foreground mb-2 font-mono uppercase">
						latency distribution
					</div>
					<LatencyBars latency={p.latency} />
				</div>

				<div>
					<nav
						class="border-border-soft flex flex-wrap items-center gap-2.5 border-b pb-1.5"
						aria-label="metric for {display.name}"
					>
						{#each view.metricTabs(summary) as tab (tab.key)}
							{@const current = view.metricFor(summary.target_id) === tab.key}
							<a
								href={tab.href}
								aria-current={current ? 'true' : undefined}
								onclick={(event) => {
									// Enhancement only. Modified clicks stay browser-native, and if the swap
									// fails the link is followed instead of leaving a dead tab.
									if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
									event.preventDefault();
									void view.swapMetric(summary.target_id, tab.key).then((ok) => {
										if (!ok) location.assign(tab.href);
									});
								}}
								class={cn(
									'text-body border-b-2 py-0.5 font-medium transition-colors',
									current
										? 'border-primary text-foreground'
										: 'hover:text-foreground text-muted-foreground border-transparent',
								)}>{tab.label}</a
							>
						{/each}
						<span class="text-caption text-muted-foreground ml-auto font-mono">
							<Hint hint={view.metricHelp(summary.target_id)}>one representative trial</Hint>
						</span>
					</nav>

					<SparkLine chart={view.chartFor(summary.target_id)} />
					<QueryMetricBars chart={view.chartFor(summary.target_id)} />
				</div>
			</div>

			<!--
				The declared configuration, folded away. It is what makes the comparison auditable, so it
				cannot leave the page — but it is reference material, not something you read on the way
				past, and inline it added a ten-row table under every target.
			-->
			<details class="border-border-soft group mt-5 border-t pt-3">
				<summary
					class="text-body text-muted-foreground hover:text-foreground flex cursor-pointer list-none items-center gap-1.5 marker:content-['']"
				>
					<span aria-hidden="true" class="inline-block transition-transform group-open:rotate-90">
						&rsaquo;
					</span>
					declared configuration and spread
				</summary>
				<div class="mt-3">
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
								<Td>{view.latencyRangeText(summary)}</Td>
							</Tr>
							<Tr>
								<Td tone="muted">saturation rps</Td>
								<Td>{fmtRps(summary.saturation.knee_rps)}</Td>
								<Td tone="muted">saturation p95</Td>
								<Td>{fmtLatency(summary.saturation.knee_p95)}</Td>
							</Tr>
							<Tr>
								<Td tone="muted">group</Td>
								<Td>{view.targetGroup(summary)}</Td>
								<Td tone="muted">target id</Td>
								<Td>{summary.target_id}</Td>
							</Tr>
						</Table.Body>
					</DataTable>
				</div>
			</details>
		</Section>
	{/each}

	{#if view.queries.length > 0}
		<Section title="Request mix" flush>
			{#snippet aside()}{view.totalQueryMix.toLocaleString()} materialized requests{/snippet}

			<div class="px-6 pt-4 pb-5">
				<Note>
					Workload composition only: the generated HTTP routes and how often each appears in the
					request list. Measured per-route throughput and latency are in each target's chart above.
				</Note>

				<ul class="text-meta mt-4 grid gap-2">
					{#each view.queries as query (query.id)}
						<li class="flex flex-wrap items-baseline justify-between gap-x-3">
							<span class="min-w-0">
								<span class="text-foreground">{query.name}</span>
								<span class="text-muted-foreground ml-1.5 font-mono">
									{query.method}
									{query.path}
								</span>
							</span>
							<span class="text-muted-foreground font-mono tabular-nums">
								{fmtPct(view.queryShare(query))} / {query.mix.toLocaleString()}
							</span>
						</li>
					{/each}
				</ul>
			</div>
		</Section>

		<Section title="Query catalog" flush>
			{#snippet aside()}{view.queries.length} operations / SQL shapes{/snippet}

			<div class="border-border border-t">
				{#each view.queries as query (query.id)}
					<details class="border-border-soft group border-b last:border-b-0">
						<summary
							class="text-meta hover:bg-muted/40 flex cursor-pointer list-none flex-wrap items-baseline justify-between gap-x-3 px-6 py-2.5 marker:content-['']"
						>
							<span class="flex items-baseline gap-1.5">
								<span
									aria-hidden="true"
									class="text-muted-foreground inline-block transition-transform group-open:rotate-90"
									>&rsaquo;</span
								>
								{query.name}
							</span>
							<span class="text-muted-foreground font-mono">
								{query.method}
								{query.path} / {query.mix.toLocaleString()}
							</span>
						</summary>
						<div class="px-6 pb-4">
							<dl class="text-meta grid grid-cols-[5.5rem_minmax(0,1fr)] gap-x-3 font-mono">
								<dt class="text-muted-foreground">params</dt>
								<dd class="break-words">
									{query.params.length ? query.params.join(', ') : 'none'}
								</dd>
							</dl>
							{#each query.sql as shape (shape.dialect + shape.text)}
								<pre
									class="border-border bg-surface-inset text-caption text-foreground-secondary mt-2 overflow-x-auto border px-3 py-2 font-mono"><code
										>{shape.text}</code
									></pre>
							{/each}
						</div>
					</details>
				{/each}
			</div>
		</Section>
	{/if}

	<Section title="About this run" flush>
		<div class="border-border border-t">
			<!--
				One fact per row, one key column and one value column. This replaced a four-column
				label/value/label/value zig-zag that made the eye jump columns to follow a single fact,
				and that carried the run id, the cohort id and the workload's sha256 as displayed text.
				Ids and hashes are provenance, not reading matter: the run id is in the URL and the
				commit is on the meta strip above.
			-->
			<DataTable>
				<Table.Body>
					{#each runFacts(manifest) as fact (fact.label)}
						<Tr>
							<Td tone="muted" class="w-56 align-top">
								{#if fact.hint}
									<Hint hint={fact.hint}>{fact.label}</Hint>
								{:else}
									{fact.label}
								{/if}
							</Td>
							<Td wrap>{fact.value}</Td>
						</Tr>
					{/each}
				</Table.Body>
			</DataTable>

			<div class="px-6 py-5">
				<Note>
					The load generator, the target server and its database all run on this one machine and
					share its cores, so target CPU and load-generator CPU come out of the same budget. Numbers
					here are comparable to other targets in this same run, and not to other runs — see <a
						class="text-link underline"
						href="/methodology">the method</a
					>.
				</Note>
			</div>
		</div>
	</Section>
</Page>
