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
					'per-target primary metrics, trial spread, confidence intervals when present, and saturation point',
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
				['throughput', 'median requests per second across trials, plus the peak sampled bucket'],
				[
					'latency',
					'mean, p50, p90, p95, p99, and p999 in milliseconds; p50 and p90 only when the artifact measured them',
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
					'whether the target issues prepared statements; rendered as a badge, and left off when the artifact does not declare it',
				],
				[
					'data access',
					'sql-roundtrip or in-process-cache; an in-process cache is listed in the ranking but never given a comparison against drizzle-rs, and keeps its own unranked section on compare',
				],
				[
					'sql variant',
					"free-form note when a target's SQL deviates from the canonical query catalog, shown under the target name",
				],
				[
					'drizzle-rs api',
					'which drizzle-rs surface a target exercises, derived from the target id suffix and the sql variant, and shown as a "sql" or "relational" tag beside the name: "sql" is the typed select builder, "relational" is the db.query(..).with(..) relational query API. They generate different SQL and are two measurements, not one. Targets from other libraries carry no tag',
				],
				[
					'fair block',
					'declared worker count, pool size, database, schema, and contract version each target must match',
				],
			],
		},
		{
			title: 'run controls',
			rows: [
				[
					'load',
					'executor, stages, duration, max virtual users, and total requests are captured per run',
				],
				[
					'dataset',
					'customers, employees, orders, suppliers, products, and details-per-order are captured per run',
				],
				[
					'runner',
					'class, os, cpu model, core count, memory, metric scopes, and peak cpu are captured per run; the os appears throughout the site as an LNX / MAC / WIN badge whose tooltip names the machine and the shard',
				],
				[
					'trials',
					'summary artifacts report the trial count, the cross-trial aggregation (median), trial spread, and optional ci95 ranges',
				],
			],
		},
		{
			title: 'interpretation',
			rows: [
				['higher is better', 'rps median and rps peak'],
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
					'saturation',
					'knee rps and knee p95 show the point where extra load starts trading throughput for latency',
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
	<title>methodology - drizzle-rs/bench</title>
</svelte:head>

<Page>
	<PageHeader title="Method">
		{#snippet subtitle()}
			fields shown here come from each run manifest and summary artifact
		{/snippet}
	</PageHeader>

	<Section title="what these numbers are not">
		<div class="measure text-prose text-foreground-secondary space-y-7">
			<section>
				<h3 class="text-heading text-foreground mb-2 font-semibold">
					The load generator shares the machine with what it measures.
				</h3>
				<p>
					Within a run, the request generator, the target HTTP server and — for embedded engines —
					the database all execute on the same CI runner and compete for the same cores. Reported
					CPU is therefore whole-host CPU, and a target that spends more CPU per request also
					starves the generator driving it. Treat the numbers as a comparison between targets under
					identical conditions, not as an absolute capacity figure for any of them.
				</p>
				<p class="mt-3">
					Linux jobs split the VM's cores between the two: the lower half runs the load generator (<code
						class="text-meta font-mono">BENCH_CPUSET_LOAD</code
					>) and the upper half runs the target and any database it spawns (<code
						class="text-meta font-mono">BENCH_CPUSET_SERVER</code
					>). This is best-effort separation, not isolation — memory bandwidth, last-level cache,
					the kernel network stack and the PostgreSQL service container are all still shared — and
					macOS and Windows jobs run unpinned. What it buys is that the generator and the system
					under test stop competing for the same cores. Pinning moves absolute numbers, so a
					baseline recorded before it is not a valid comparison point for a pinned run.
				</p>
			</section>

			<section>
				<h3 class="text-heading text-foreground mb-2 font-semibold">
					Cross-family rows usually come from different VMs.
				</h3>
				<p>
					A benchmark set is assembled from several CI jobs — typically one per database family —
					and each job runs on its own freshly allocated runner. Two rows in the same set are only
					directly comparable when they share a shard. Every page that lists targets marks which
					machine each row came off with a three-letter badge —
					<code class="text-meta font-mono">LNX</code>, <code class="text-meta font-mono">MAC</code>
					or <code class="text-meta font-mono">WIN</code> — whose tooltip carries the full OS name and
					the shard timestamp. Rows with different badges were measured on different hardware, and the
					ranking is one table across every database precisely so that fact is visible on the row rather
					than implied by which section a row was filed under.
				</p>
				<p class="mt-3">
					Publish-class runs are the exception for PostgreSQL: there the three PostgreSQL families
					run back to back inside a single job, against a single PostgreSQL service, so their rows
					share a shard and are comparable to each other. Preview and pull-request runs keep those
					families parallel on separate VMs. The shard label is what tells the two apart — rows that
					share one were measured on the same machine.
				</p>
			</section>

			<section>
				<h3 class="text-heading text-foreground mb-2 font-semibold">
					Pacing imposes a ceiling on offered load.
				</h3>
				<p>
					A closed-loop suite with per-request think time bounds the maximum request rate the
					generator can offer at roughly
					<code class="text-meta font-mono">VUs / (mean think time + mean service time)</code>. A
					target fast enough to sit under that ceiling reports the ceiling, not its capacity; a
					throughput number close to that bound is a statement about the load profile, not about the
					target. The unpaced throughput suite exists for saturation measurements.
				</p>
			</section>

			<section>
				<h3 class="text-heading text-foreground mb-2 font-semibold">
					In-process caches are not doing database work.
				</h3>
				<p>
					Targets declaring <code class="text-meta font-mono">data_access: in-process-cache</code>
					answer from a replicated local copy, with no per-request round trip to a database. On the ranking
					they appear in the one table like everything else, with "in-memory cache — no per-request DB
					work" spelled out under the name and a dash instead of a comparison against drizzle-rs, because
					a cache hit and a query are not the same measurement. On
					<a class="text-link underline" href="/compare">compare</a> they keep their own unranked section.
				</p>
			</section>
		</div>
	</Section>

	<Section title="how values are aggregated">
		<div class="measure text-prose text-foreground-secondary space-y-5">
			<p>
				Each target is measured over <em>n</em> trials. The summary artifact spells its cross-trial
				keys <code class="text-meta font-mono">avg</code>, but the value stored there is the
				<strong class="text-foreground font-medium">median across trials</strong>, which is why this
				dashboard labels those columns <code class="text-meta font-mono">median</code>. Where a
				label reads <code class="text-meta font-mono">lat mean</code>, the number is the median
				across trials of each trial's mean latency — the mean is inside the trial, the median is
				across them.
			</p>
			<p>
				Percentiles are computed from the trial's merged raw samples. Runs published before the
				runner measured real percentiles carry no <code class="text-meta font-mono">p50</code>; for
				those runs the <code class="text-meta font-mono">p90</code> column is hidden rather than shown,
				because the value it held was interpolated rather than measured.
			</p>
			<p>
				<code class="text-meta font-mono">peak core</code> is the highest single-core utilization
				observed, not spare capacity. A high value means the run was CPU-bound on one core. Where a
				run also reports <code class="text-meta font-mono">mean-core peak</code>, that is the
				mean-across-cores figure the runner's publish gate is written against.
			</p>
		</div>
	</Section>

	{#each REFERENCE as group (group.title)}
		<Section title={group.title}>
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
		</Section>
	{/each}

	<Section title="local commands">
		<pre
			class="border-border bg-muted text-meta overflow-x-auto border px-4 py-3 font-mono leading-relaxed"><span
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
