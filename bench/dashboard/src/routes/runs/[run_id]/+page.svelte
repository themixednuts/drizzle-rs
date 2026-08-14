<script lang="ts">
	import BoxWhisker from '#lib/components/BoxWhisker.svelte';
	import CapacityFigure from '#lib/components/CapacityFigure.svelte';
	import HarnessStrip from '#lib/components/HarnessStrip.svelte';
	import LatencyBars from '#lib/components/LatencyBars.svelte';
	import OverlayChart from '#lib/components/OverlayChart.svelte';
	import Replay from '#lib/components/Replay.svelte';
	import SaturationCurve from '#lib/components/SaturationCurve.svelte';
	import QueryMetricBars from '#lib/components/QueryMetricBars.svelte';
	import SparkLine from '#lib/components/SparkLine.svelte';
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import MetricGrid from '#lib/components/MetricGrid.svelte';
	import MetricTile from '#lib/components/MetricTile.svelte';
	import Note from '#lib/components/Note.svelte';
	import Hint from '#lib/components/Hint.svelte';
	import ApiTag from '#lib/components/ApiTag.svelte';
	import OsBadge from '#lib/components/OsBadge.svelte';
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
		runStamp,
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
	<!--
		No status badge. Every run that renders this page succeeded — a failed one has no summaries to
		draw — so the badge was a green tick beside a page that could not exist without it. The runs
		list is where status distinguishes one row from another, and it still carries it there.
	-->
	<PageHeader title={view.runName} back={{ href: '/runs', label: 'all runs' }} />

	<!--
		The comp's meta strip: the facts that identify this run, on one mono line under the title
		instead of scattered across a metadata table and a KPI header. Everything the strip does not
		carry is still in "About this run" below.
	-->
	<div
		class="border-border text-label text-muted-foreground mt-4 flex flex-wrap gap-x-7 gap-y-2 border-b pb-5 font-mono"
	>
		<span>commit {shortHash(manifest.git)}</span>
		<span>{fmtDate(manifest.start)}</span>
		<span>{fmtDuration(manifest.start, manifest.end)}</span>
		<span>{manifest.trials.count} trials each</span>
		<!-- The machine this run's every number came off, as the same badge the ranking prints on
		     each of its rows. -->
		<span class="inline-flex items-center gap-x-2">
			<OsBadge os={runner.os} detail="shard {runStamp(manifest.run_id)}" />
			{runner.cores}-core
		</span>
		{#if classLabel(runner.class)}<span>{classLabel(runner.class)}</span>{/if}
	</div>

	<!--
		The harness legend, above the targets it governs. Most runs are one family and this is one
		line, which is still worth stating: a reader arriving from the ranking's per-database strip
		should find the same three facts here, attached to the run that produced them.
	-->
	<HarnessStrip rows={view.harnessRows} />

	<!--
		The run itself, before any of the reductions below it.

		Every section under this one is a target's numbers after five trials have been collapsed into
		a median. This is the ramp those numbers came off: load climbing along the bottom, each
		target's rate drawing itself across, and a readout that follows the playhead. Where one line
		flattens and another keeps climbing is a finding no table on this page can state.
	-->
	{#if view.replay}
		<Section>
			<Replay replay={view.replay} />
		</Section>
	{:else}
		<!-- No load ramp was recorded, so there is nothing to play. The same series against elapsed
		     time is still worth drawing. -->
		<Section>
			<OverlayChart chart={view.overlays.rps} height={190} />
		</Section>

		<Section>
			<OverlayChart chart={view.overlays.latency} height={150} />
		</Section>
	{/if}

	<!-- One section per target, in throughput order, with the accent edge on ours. -->
	{#each view.sortedSummaries as summary (summary.target_id)}
		{@const p = summary.primary}
		{@const meta = view.targetMeta(summary.target_id)}
		{@const display = view.targetDisplay(summary.target_id)}
		{@const ours = view.isBaseline(summary)}
		<Section class={cn(ours && 'border-l-signal border-l-[3px]')}>
			<div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
				<h2 class={cn('text-heading font-semibold', ours && 'text-signal-ink')}>{display.name}</h2>
				<ApiTag api={display.api} />
				<span class="text-meta text-foreground-secondary">{display.note}</span>
				<span class="text-label text-muted-foreground ml-auto font-mono">
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

			<!--
				The headline pair, and the whole point of the layout: peak throughput is the capacity
				claim and the paced rate is not, so they sit side by side, at the same size, each
				carrying the words that say which one it is. `CapacityFigure` is the only thing that can
				draw the left-hand number and it always prints the objective under it, so there is no
				arrangement of this page where a bare rps figure could be mistaken for the other.
			-->
			{#if view.hasCapacity}
				<div class="border-border-soft mt-5 grid gap-6 border-b pb-5 sm:grid-cols-2">
					<div>
						<div class="text-micro text-muted-foreground font-mono uppercase">peak throughput</div>
						<div class="mt-2">
							<CapacityFigure capacity={view.capacity(summary)} size="lead" />
						</div>
					</div>
					<div>
						<div class="text-micro text-muted-foreground font-mono uppercase">
							<Hint
								hint="The paced suite's request rate: the generator offers a fixed load with per-request think time, so this measures latency at a known rate and cannot exceed VUs / think time. It is not a capacity figure."
							>
								throughput at fixed load
							</Hint>
						</div>
						<div class="text-figure mt-2 font-mono font-medium tabular-nums">
							{fmtRps(p.rps.avg)}
						</div>
						<div class="text-body text-muted-foreground mt-1">
							median across trials · busiest second {fmtRps(p.rps.peak)}
						</div>
					</div>
				</div>
			{/if}

			<div class="mt-5">
				<MetricGrid>
					{#if !view.hasCapacity}
						<MetricTile
							label="requests/sec"
							value={fmtRps(p.rps.avg)}
							detail="busiest second {fmtRps(p.rps.peak)}"
							hint="median requests/second across trials, under the paced suite's fixed offered load"
						/>
					{/if}
					<MetricTile
						label="typical latency"
						value={fmtLatency(p.latency.avg)}
						detail="median across trials"
						hint="median across trials of each trial's mean latency"
					/>
					<!--
						Labelled for what it is. This is the whole-ramp percentile: past the point where a
						target stops keeping up, the ramp's extra load is queueing, so the figure carries the
						queue as well as the work. The sustained-load reading is the one that measures the
						target, and it lives on the ranking.
					-->
					<MetricTile
						label="slowest 5%"
						value={fmtLatency(p.latency.p95)}
						detail="whole ramp · p99 {fmtLatency(p.latency.p99)}"
						hint="p95 merged across every hold plateau of the ramp, up to 3000 concurrent, median across trials. Past a target's throughput ceiling the ramp contributes queueing rather than work, so this is not the target's service latency."
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

			<!--
				The evidence for the headline, immediately under it. A peak is a claim about a shape —
				throughput flattening while latency turns up — and the shape is the thing a reader can
				check for themselves.
			-->
			{#if view.curve(summary)}
				<SaturationCurve curve={view.curve(summary)!} targetName={display.name} />
			{:else if view.hasCapacity}
				<p class="measure text-meta text-muted-foreground mt-6">
					Other targets in this run were measured for peak throughput; this one was not, so there is
					no ramp to draw for it.
				</p>
			{/if}

			<!--
				The trials behind the headline. Every number in the grid above is a median across five
				runs, and how far those five ranged is what decides whether a difference between two
				targets is a result — so it is drawn here rather than left as a text range folded into
				the configuration table below.
			-->
			{@const figure = view.spreadFigure(summary)}
			<div class="border-border-soft mt-6 border-t pt-4">
				<div class="text-micro text-muted-foreground font-mono uppercase">
					<Hint
						hint="Whiskers reach the slowest and fastest trial. The box spans the middle two quartiles, and is drawn only where the run recorded them."
					>
						rate across trials
					</Hint>
				</div>
				<div class="mt-2 grid items-center gap-x-5 gap-y-2 lg:grid-cols-[minmax(0,1fr)_auto]">
					<BoxWhisker
						box={figure.box}
						extent={figure.extent}
						label={view.spreadLabel(summary)}
						accent={ours}
					/>
					<span class="text-meta text-foreground-secondary font-mono tabular-nums">
						{view.spreadLabel(summary)}
					</span>
				</div>
			</div>

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
						<span class="text-label text-muted-foreground ml-auto font-mono">
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
					<DataTable minWidth="min-w-[40rem]">
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
							<!-- The rate's spread is drawn above; only the p95 range is left to state here. -->
							<Tr>
								<Td tone="muted">p95 across trials</Td>
								<Td colspan={3}>{view.latencyRangeText(summary)}</Td>
							</Tr>
							<!--
								The harness this target's whole database family ran under, beside the target's own
								declared worker/pool numbers above. Two rows on one database share this line; two
								rows on different databases do not, and that difference is the stack rather than
								the library.

								This replaced two rows headed "saturation rps" and "saturation p95". Those carried
								the runner's old knee heuristic — computed off the paced run, and falling back to
								its busiest bucket whenever it found no knee — under a name that now means the
								measured capacity figure at the top of this section. A paced number wearing that
								word is exactly what the saturation suite exists to stop, so it is gone rather
								than renamed.
							-->
							<Tr>
								<Td tone="muted">
									<Hint
										hint="Workers, pool size and database tuning, enforced identical for every target on this database. Targets on other databases run their own harness by design — that difference is the stack comparison."
									>
										family harness
									</Hint>
								</Td>
								<Td colspan={3}>
									{#if view.harnessLine(summary)}
										{@const harness = view.harnessLine(summary)!}
										<span class="font-mono">{harness.text}</span>
										{#if harness.identical}
											<span class="text-muted-foreground ml-2"
												>verified identical within family</span
											>
										{:else}
											<span class="text-negative ml-2">not identical within family</span>
										{/if}
									{:else}
										<span class="text-muted-foreground italic">not declared by this run</span>
									{/if}
								</Td>
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

			<div class="border-border-soft border-t">
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
									class="bg-surface-inset text-label text-foreground-secondary mt-2 overflow-x-auto rounded-sm px-3 py-2 font-mono"><code
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
		<div class="border-border-soft border-t">
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
						class="underline underline-offset-2"
						href="/methodology">the method</a
					>.
				</Note>
			</div>
		</div>
	</Section>
</Page>
