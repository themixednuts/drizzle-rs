<script lang="ts">
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import DataTable from '#lib/components/data/DataTable.svelte';
	import Td from '#lib/components/data/Td.svelte';
	import Tr from '#lib/components/data/Tr.svelte';
	import * as Table from '#lib/components/ui/table/index.js';

	/**
	 * Reference tables. Each row is `term -> what it means`, which is a definition list rendered as
	 * a table because the terms line up with the column headers used elsewhere on the site.
	 */
	const REFERENCE = [
		{
			title: 'data model',
			rows: [
				['index.json', 'run list with suite, status, commit, time window, class, and target ids'],
				[
					'manifest.json',
					'run configuration, runner, load profile, dataset shape, artifacts, and target list',
				],
				[
					'summary.json',
					'per-target primary metrics, trial spread, confidence intervals when present, and, on runs that measured it, the saturation block. That block holds the objective, the outcome, the peak or lower bound, and the full concurrency curve',
				],
				[
					'timeseries.json',
					'per-bucket rps, errors, latency percentiles, host cpu samples, process-tree memory when present, and route-level query metrics',
				],
			],
		},
		{
			title: 'reported metrics',
			rows: [
				[
					'peak throughput',
					'the saturation suite\'s capacity figure: the fastest concurrency step that held the latency objective and stayed inside the error limit, ties going to the lower concurrency. Always printed with the objective beside it ("at p99 < 50 ms") and never without it',
				],
				[
					'throughput at fixed load',
					"the paced suite's median requests per second across trials. A latency-at-a-known-rate reading, bounded above by the load profile, and not a capacity figure",
				],
				[
					'busiest second',
					'the fastest single sample bucket of a paced run. A momentary rate inside a fixed-load run; it was previously labelled "peak throughput", which now means the capacity figure above',
				],
				[
					'latency at load',
					'p95 read at the highest rung of the ramp where the target still served the load it was offered. This is the figure that measures the target',
				],
				[
					'whole-ramp latency',
					'the same run, with percentiles merged across every hold plateau up to 3000 concurrent. Past the point where a target stops keeping up, the extra load is queueing, so this figure carries the queue as well as the work. It is the upstream method, kept so the throughput beside it stays comparable',
				],
				[
					'cpu',
					'median across trials of mean-across-cores host utilization, plus the peak single-core utilization',
				],
				[
					'memory',
					'median and peak target process-tree resident memory in MB when a target process can be sampled',
				],
				['errors', 'errored requests as a fraction of total requests'],
			],
		},
		{
			title: 'target declarations',
			rows: [
				[
					'prepared',
					'whether the target issues prepared statements. It rides on the target note line, and drops out entirely when the artifact does not declare it',
				],
				[
					'data access',
					'sql-roundtrip or in-process-cache. A cache-backed target still ranks, but carries a dash where the comparison against drizzle-rs would go, because it is not doing the same work',
				],
				[
					'sql variant',
					"free-form note when a target's SQL deviates from the canonical query catalog, shown under the target name",
				],
				[
					'drizzle-rs api',
					'which drizzle-rs API a target exercises, derived from the target id suffix and the sql variant, and shown as a "sql" or "relational" tag beside the name: "sql" is the typed select builder, "relational" is the db.query(..).with(..) relational query API. They generate different SQL and are two measurements, not one. Targets from other libraries carry no tag',
				],
				[
					'fair block',
					'declared worker count, pool size, database, schema, and contract version each target must match',
				],
				[
					'comparison group',
					'the set of targets claiming to be directly comparable, declared per target as fair.family. Usually a database, and split where the harness cannot be equalised. sqlite (Rust) and sqlite-ts (Bun) are both SQLite, and they are two groups. Enforcement and the "vs …" delta are scoped to the group; the table and the database column are not, so a split never hides a row',
				],
				[
					'group harness',
					'the workers, pool size and tuning a whole comparison group ran under, recorded once per group in the manifest, plus whether within-group identity was verified and which targets if any were exempted from that check. It sits behind "How each engine was configured" below the ranking, and inside each row\'s own detail; a group with no declaration reads "harness not declared" rather than inheriting one',
				],
			],
		},
		{
			title: 'run controls',
			rows: [
				[
					'load',
					'the run records the executor, stages, duration, max virtual users and total requests',
				],
				[
					'dataset',
					'the run records customers, employees, orders, suppliers, products and details-per-order',
				],
				[
					'runner',
					'the run records class, os, cpu model, core count, memory, metric scopes and peak cpu. The os appears throughout the site as an LNX / MAC / WIN badge whose tooltip names the machine and the shard',
				],
				[
					'trials',
					'a summary artifact reports the trial count, the cross-trial aggregation (median), the trial spread and optional ci95 ranges',
				],
			],
		},
		{
			title: 'interpretation',
			rows: [
				['higher is better', 'peak throughput, throughput at fixed load, and busiest second'],
				['lower is better', 'latency, cpu, memory, and error rate'],
				[
					'direction',
					'an up arrow means the drizzle target is ahead on the leaderboards, and improved against the previous set on trends; a down arrow is the opposite in both. Colour repeats what the arrow and the sign already say',
				],
				[
					'box plot',
					'drawn only from recorded quartiles. When an artifact records min/max but no quartiles, the bar shows the range and the median tick with no box; when it records neither, a single tick is shown',
				],
				[
					'saturation outcome',
					'one of four states, never a substituted number: a measured peak; "at least N req/s · knee not reached" for a ramp that ended before the target did; "never met the p99 target" when even the smallest step breached the objective; and "not measured" for runs that did not run the suite',
				],
				[
					'disqualified step',
					"a step of the ramp whose error rate exceeded the run's limit. It is measured, drawn on the curve struck through, and listed in the step table with its reason. It can never be chosen as the peak",
				],
				[
					'rank',
					'position in the one table, across every database. There is no rank column, because the table is sorted and a reader can see where a row sits; the number is in the accessible name for readers who cannot. Under the peak-throughput order a row whose peak was never measured sorts below every row that has one, because position on a ranked table reads as a claim',
				],
				[
					'caveat',
					'synthetic benchmarks are ceilings for a workload, not predictions for every application shape',
				],
			],
		},
	];
</script>

<svelte:head>
	<title>Method / drizzle-rs benchmarks</title>
</svelte:head>

<Page>
	<PageHeader title="Method">
		{#snippet subtitle()}
			every field below comes from a run manifest or a summary artifact
		{/snippet}
	</PageHeader>

	<!--
		Four claims, one sentence of consequence each. This section used to run four headings
		deep with two dense paragraphs under every one, which is the shape of notes written by
		someone who already knows the answer. A reader arriving here wants to know what they
		may not conclude from the numbers; the reasoning behind each limit is in the repo.
	-->
	<Section title="What these numbers are not">
		<dl class="measure text-prose text-foreground-secondary space-y-5">
			<div>
				<dt class="text-foreground font-semibold">Not absolute capacity.</dt>
				<dd>
					The load generator, the target and any embedded engine share one CI runner, so the CPU
					figure covers the whole host. Compare these rows against each other, not against a number
					you measured on your own hardware.
				</dd>
			</div>

			<div>
				<dt class="text-foreground font-semibold">Pinned on Linux, unpinned elsewhere.</dt>
				<dd>
					Linux jobs give the lower half of the cores to the generator and the upper half to the
					system under test, with an out-of-process database taking a slice of that upper half.
					Cache, memory bandwidth and the network stack stay shared, so the two halves still
					interfere. macOS and Windows expose no usable affinity API and run unpinned.
				</dd>
			</div>

			<div>
				<dt class="text-foreground font-semibold">One machine per ranking, not per row.</dt>
				<dd>
					Published rankings run every family an operating system can host back to back on one VM,
					so the rows in one are comparable. Other runs keep families on separate VMs; the
					<code class="text-meta font-mono">LNX</code>/<code class="text-meta font-mono">MAC</code
					>/<code class="text-meta font-mono">WIN</code> badge and its shard tell the two apart.
				</dd>
			</div>

			<div>
				<dt class="text-foreground font-semibold">Paced throughput is the generator's ceiling.</dt>
				<dd>
					With think time, offered load caps at roughly
					<code class="text-meta font-mono">VUs / think time</code>, and every healthy target lands
					within a few percent of it. That ceiling is why capacity gets its own suite.
				</dd>
			</div>

			<div>
				<dt class="text-foreground font-semibold">Caches are not doing database work.</dt>
				<dd>
					Targets marked <code class="text-meta font-mono">in-process-cache</code> answer from a local
					replica with no round trip, so they carry a dash instead of a comparison against drizzle-rs.
				</dd>
			</div>
		</dl>
	</Section>

	<Section title="Two suites, two headlines">
		<dl class="measure text-prose text-foreground-secondary space-y-5">
			<div>
				<dt class="text-foreground font-semibold">Throughput at fixed load, paced.</dt>
				<dd>
					Virtual users send a request, wait a think time, send the next. The ramp is a port of the
					profile drizzle-benchmarks publishes under — the same thirty stages to 3000 concurrent,
					the same 0–375&nbsp;ms think-time staircase averaging 187.5&nbsp;ms. It is not a capacity
					figure.
				</dd>
			</div>
			<div>
				<dt class="text-foreground font-semibold">Latency, read where the target kept up.</dt>
				<dd>
					A closed ramp cannot offer more than a target can serve, so past its ceiling every extra
					virtual user becomes queue depth and the reported latency measures the ramp rather than
					the library. So latency is read at the highest rung the target still served in full —
					where its throughput still rose in step with the load offered to it. Every rung publishes
					what it was offered, what it served, and the ratio between them, so the reading can be
					checked against its own working. The cut between keeping up and falling behind is drawn
					from the recorded runs rather than chosen, and it is narrow: a target within about a
					percent of it can read at a different rung between runs.
				</dd>
			</div>
			<div>
				<dt class="text-foreground font-semibold">Peak throughput, from saturation.</dt>
				<dd>
					The same workload with think time removed, stepped over concurrency: hold, measure steady
					state, step up. The headline is the fastest step that held the latency objective and
					stayed inside the error limit. A step that broke the error limit is disqualified, and the
					curve strikes it through. Every step publishes its own throughput, percentiles, errors and
					CPU, not only the winning one.
				</dd>
			</div>
		</dl>
	</Section>

	<!--
		These four outcomes are the contract, so this list has to track `saturation.rs` exactly.
		It previously described the outcome as turning on whether the last step breached the
		objective, which stopped being true when the rule moved to throughput turning over.
	-->
	<Section title="When there is no peak">
		<div class="measure text-prose text-foreground-secondary space-y-4">
			<p>
				A capacity measurement can fail to produce a number. When it does, nothing stands in for it.
				Not a zero, not the top of the ramp, not the paced number wearing the other one's name.
			</p>
			<dl class="space-y-4">
				<div>
					<dt class="text-foreground font-medium">A peak, at a stated objective</dt>
					<dd class="mt-1">
						Throughput climbed to a maximum and then fell away from it, so the ramp found the
						ceiling from both sides. It prints with its objective and the concurrency that reached
						it. This is the only outcome that earns a rank.
					</dd>
				</div>
				<div>
					<dt class="text-foreground font-medium">"at least N req/s · knee not reached"</dt>
					<dd class="mt-1">
						Throughput was still flat or climbing when the ramp ended, so the best step is a floor
						rather than a ceiling. Shown faint, with "at least", ranked below every measured peak. A
						maximum sitting on the ramp's first step counts here too, because nothing below it was
						tried.
					</dd>
				</div>
				<div>
					<dt class="text-foreground font-medium">"never met the p99 target"</dt>
					<dd class="mt-1">
						Even the smallest step breached the objective. The curve is still drawn, because how far
						over it landed is the useful part.
					</dd>
				</div>
				<div>
					<dt class="text-foreground font-medium">"not measured"</dt>
					<dd class="mt-1">
						The run predates the saturation suite or skipped it. Older runs carry a differently
						defined field of the same name; this dashboard does not read it.
					</dd>
				</div>
			</dl>
		</div>
	</Section>

	<Section title="Fair means two different things">
		<dl class="measure text-prose text-foreground-secondary space-y-5">
			<div>
				<dt class="text-foreground font-semibold">
					Inside a comparison group, identical and enforced.
				</dt>
				<dd>
					Every target in a group runs the same workers, pool and tuning, so the gap between two of
					its rows is the library. The runner fails a run whose group disagrees rather than
					publishing a quietly unequal comparison, and each row's "vs" figure is scoped to its own
					group.
				</dd>
			</div>
			<div>
				<dt class="text-foreground font-semibold">A group is not always a database.</dt>
				<dd>
					It splits wherever one harness cannot fit both sides.
					<code class="text-meta font-mono">bun:sqlite</code> is synchronous on a single-threaded runtime,
					so handing it the Rust stack's pool of eight would be fiction. It gets its own SQLite group
					with drizzle-orm. Both still appear in the one table under SQLite. The split changes what a
					row is measured against, never whether it is shown.
				</dd>
			</div>
			<div>
				<dt class="text-foreground font-semibold">Across groups, different and declared.</dt>
				<dd>
					Forcing an embedded engine and a client/server engine into one configuration does not make
					them comparable. It makes them equally hobbled. Each stack runs in the shape it is
					deployed in, and every run records what that was, under <em>Run configuration</em> below the
					ranking and inside each row's own detail. A group that declared nothing says so instead of borrowing
					a neighbour's.
				</dd>
			</div>
		</dl>
	</Section>

	<Section title="How trials become one number">
		<dl class="measure text-prose text-foreground-secondary space-y-5">
			<div>
				<dt class="text-foreground font-semibold">Median across trials.</dt>
				<dd>
					The artifact spells the key <code class="text-meta font-mono">avg</code>, but stores the
					median, so these columns read <code class="text-meta font-mono">median</code>. Where a
					label says <code class="text-meta font-mono">lat mean</code>, the mean is inside each
					trial and the median is across them.
				</dd>
			</div>
			<div>
				<dt class="text-foreground font-semibold">Percentiles from merged raw samples.</dt>
				<dd>
					Runs predating real percentiles carry no <code class="text-meta font-mono">p50</code>, and
					their <code class="text-meta font-mono">p90</code> column is hidden rather than shown, because
					the value was interpolated.
				</dd>
			</div>
			<div>
				<dt class="text-foreground font-semibold">CPU is load, not headroom.</dt>
				<dd>
					<code class="text-meta font-mono">peak core</code> is the busiest single core; a high
					value means the run was CPU-bound on one.
					<code class="text-meta font-mono">mean-core peak</code>, where present, is the figure the
					publish gate is written against.
				</dd>
			</div>
		</dl>
	</Section>

	<!--
		The field glossary is 850 words — half this page — and it is lookup material: nobody reads
		a glossary top to bottom, they arrive at it holding one term. Closed by default it costs a
		reader nothing and stays one click from anyone who needs it, which is the same reason the
		ranking's run configuration is a disclosure rather than a banner.
	-->
	<details class="bg-card mt-8 rounded-md">
		<summary
			class="text-meta text-foreground-secondary hover:text-foreground cursor-pointer px-4 py-2.5 transition-colors"
		>
			Field reference
		</summary>
		<div class="border-border-soft space-y-6 border-t px-4 py-4">
			{#each REFERENCE as group (group.title)}
				<section>
					<h2 class="text-micro text-muted-foreground type-narrow mb-2 font-mono uppercase">
						{group.title}
					</h2>
					<DataTable>
						<Table.Body>
							{#each group.rows as [term, definition] (term)}
								<Tr>
									<Td tone="muted" class="w-40 align-top">{term}</Td>
									<Td wrap class="text-foreground-secondary">{definition}</Td>
								</Tr>
							{/each}
						</Table.Body>
					</DataTable>
				</section>
			{/each}
		</div>
	</details>

	<Section title="Running it yourself">
		<pre
			class="bg-surface-inset text-meta overflow-x-auto rounded-sm px-4 py-3 font-mono leading-relaxed"><span
				class="text-muted-foreground"># run Rust benchmarks</span
			>
cargo bench --features "rusqlite,uuid"

<span class="text-muted-foreground"
				># run the dashboard with Cloudflare bindings and the edge cache</span
			>
cd bench/dashboard
bun run cf:dev</pre>
	</Section>
</Page>
