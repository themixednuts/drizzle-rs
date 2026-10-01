<script lang="ts">
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import DataTable from '#lib/components/data/DataTable.svelte';
	import Td from '#lib/components/data/Td.svelte';
	import Tr from '#lib/components/data/Tr.svelte';
	import * as Table from '#lib/components/ui/table/index.js';
	import ArchitectureLegend from '#lib/components/ArchitectureLegend.svelte';
	import { ARCHITECTURE_ORDER, ARCHITECTURES } from '#lib/target-display';

	/** The legend, without counts: on this page it describes the vocabulary, not one run. */
	const ARCH_ENTRIES = ARCHITECTURE_ORDER.map((arch) => ({
		arch,
		...ARCHITECTURES[arch],
		count: 0,
	}));

	const METRICS = [
		[
			'peak',
			'Fastest step of the unpaced ramp that met the latency objective. "at least N" means the ramp ended before throughput turned over.',
		],
		[
			'paced',
			'Requests per second under the paced load (drizzle-benchmarks ramp, with think time).',
		],
		['p95', 'Read at the same offered load for every target, before any target saturates.'],
		['vs #1', 'Distance to the leader on the sorted column; below it, to the row above.'],
		['trials', 'Every figure is the median across trials; the row detail shows the spread.'],
	];

	const LIMITS = [
		'One CI VM per ranking; compare rows, not against your own hardware.',
		'Linux pins load and target to separate cores; macOS and Windows cannot.',
		'Errored rows (> 0.5% failures) sort last.',
		'Synthetic workload: a ceiling for this shape, not a prediction for your app.',
	];

	/** The HTTP contract, as a reader would recognise it. Paths match `bench/runner/src/parity.rs`. */
	const ROUTES = [
		['/customers, /employees, /suppliers, /products', 'paginated lists'],
		['/customer-by-id, /supplier-by-id', 'single row by primary key'],
		['/employee-with-recipient, /product-with-supplier', 'one-to-one join'],
		['/order-with-details, /orders-with-details', 'one-to-many, single and paginated'],
		['/order-with-details-and-products', 'two-level join'],
		['/search-customer, /search-product', 'substring search (checked for parity, not timed)'],
	];

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
					'sql-roundtrip, in-database or in-process-cache. in-database means the route logic runs as module code inside the database (SpacetimeDB). A cache-backed target still ranks, but carries a dash where the comparison against drizzle-rs would go, because it is not doing the same work',
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
	<PageHeader title="Method" />

	<Section title="What is measured">
		<p class="measure text-body text-foreground-secondary">
			Every target is an HTTP server answering the same routes over the same seeded Northwind data.
			Responses are checked for parity before anything is timed.
		</p>
		<DataTable class="mt-4">
			<Table.Body>
				{#each ROUTES as [paths, shape] (paths)}
					<Tr>
						<Td class="w-1/2 align-top font-mono">{paths}</Td>
						<Td wrap class="text-foreground-secondary">{shape}</Td>
					</Tr>
				{/each}
			</Table.Body>
		</DataTable>
	</Section>

	<Section title="How targets reach data">
		<ArchitectureLegend entries={ARCH_ENTRIES} counts={false} />
		<p class="measure text-meta text-muted-foreground mt-4">
			SpacetimeDB runs three ways: module procedures (its intended design), SQL over PGWire, and the
			SDK's client replica.
		</p>
	</Section>

	<Section title="Metrics">
		<dl class="text-body grid grid-cols-[8rem_1fr] gap-x-6 gap-y-2.5">
			{#each METRICS as [term, definition] (term)}
				<dt class="text-foreground font-mono">{term}</dt>
				<dd class="text-foreground-secondary">{definition}</dd>
			{/each}
		</dl>
	</Section>

	<Section title="Limits">
		<ul class="text-body text-foreground-secondary list-disc space-y-1.5 pl-5">
			{#each LIMITS as limit (limit)}
				<li>{limit}</li>
			{/each}
		</ul>
	</Section>

	<details class="bg-card mt-4 rounded-md">
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

	<Section title="Run it">
		<pre
			class="bg-surface-inset text-meta overflow-x-auto rounded-sm px-4 py-3 font-mono leading-relaxed">cargo build --release -p bench-runner
export BENCH_RUNNER_BIN=$PWD/target/release/bench-runner
$BENCH_RUNNER_BIN run --suite throughput-http \
  --workload bench/spec/workload.preview.v1.json \
  --targets bench/spec/targets.sqlite.v1.json \
  --requests bench/spec/requests.empty.v1.json \
  --out bench-out/sqlite --cohort-id local --class small --publish
cd bench/dashboard && BENCH_DATA_DIR=../../bench-out/sqlite bun run dev</pre>
	</Section>
</Page>
