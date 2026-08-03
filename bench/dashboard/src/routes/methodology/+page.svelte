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
					'per-target primary metrics, trial spread, confidence intervals when present, and — on runs that measured it — the saturation block: the objective, the outcome, the peak or lower bound, and the full concurrency curve',
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
				[
					'comparison group',
					'the set of targets claiming to be directly comparable, declared per target as fair.family. Usually a database, and split where the harness cannot be equalised — sqlite (Rust) and sqlite-ts (Bun) are both SQLite and are two groups. Enforcement and the "vs …" delta are scoped to the group; the table and the database column are not, so a split never hides a row',
				],
				[
					'group harness',
					'the workers, pool size and tuning a whole comparison group ran under, recorded once per group in the manifest, plus whether within-group identity was verified and which targets if any were exempted from that check. Shown as a strip above the ranking and on each row; a group with no declaration reads "harness not declared" rather than inheriting one',
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
					'one of four states, never a substituted number: a measured peak; "at least N req/s — knee not reached" for a ramp that ended before the target did; "never met the p99 target" when even the smallest step breached the objective; and "not measured" for runs that did not run the suite',
				],
				[
					'disqualified step',
					"a step of the ramp whose error rate exceeded the run's limit. It is measured, drawn on the curve struck through, and listed in the step table with its reason — and it can never be chosen as the peak",
				],
				[
					'rank',
					'01..N across every database in one table. Under the peak-throughput order only rows with a measured peak are numbered; a lower bound or an unmeasured row shows a dash and sorts below every measured one, because position on a ranked table reads as a claim',
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
					target.
				</p>
				<p class="mt-3">
					This is not a small effect. Under the paced suite every healthy target lands within a few
					percent of the same number, because the number is mostly the generator's sleep timer: a
					tenfold difference in service time moves it by well under a tenth. That is why capacity
					has its own suite, described below, and why the two are never averaged into one figure.
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

	<Section title="two suites, two headlines">
		<div class="measure text-prose text-foreground-secondary space-y-7">
			<p>
				Every target is measured twice, by two load profiles that answer two different questions.
				Their numbers are never averaged together and never share a column, because a reader who
				confuses them draws exactly the wrong conclusion.
			</p>

			<section>
				<h3 class="text-heading text-foreground mb-2 font-semibold">
					Throughput at fixed load — the paced suite.
				</h3>
				<p>
					Virtual users send a request, wait a think time, and send the next. The generator offers a
					fixed amount of work and the measurement is how well the target keeps up with it: latency
					at a known rate. This is the profile drizzle-benchmarks publishes its TypeScript numbers
					under, so it is what makes those numbers comparable to these. It is a good latency
					measurement and, for the reason above, it cannot be a capacity measurement.
				</p>
			</section>

			<section>
				<h3 class="text-heading text-foreground mb-2 font-semibold">
					Peak throughput — the saturation suite.
				</h3>
				<p>
					The same workload with the think time removed, run as a stepped ramp: hold a fixed number
					of concurrent requests, measure steady state, step up, repeat. With no think time, N
					virtual users are N requests in flight, so the ramp is over concurrency and throughput is
					whatever the target can actually turn over.
				</p>
				<p class="mt-3">
					The headline is the fastest step whose steady-state
					<code class="text-meta font-mono">p99</code> stayed under the latency objective
					<em>and</em> whose error rate stayed inside the run's limit. Both conditions matter: a step
					that returned errors faster is not a faster step, so a step over the error limit is disqualified
					from being the peak, and the disqualification is recorded and shown rather than quietly skipped.
				</p>
				<p class="mt-3">
					Which of the three outcomes a run gets is decided separately, by whether its <em>last</em>
					step still held the objective. That is deliberately not the same question as where the maximum
					landed: a ramp can peak early, flatten, and still finish without breaching, and that is "knee
					not reached" rather than a measured limit.
				</p>
				<p class="mt-3">
					The whole ramp is published, not just the winning step — every step's concurrency,
					throughput, percentiles, error rate and CPU. That curve is on each target's section of a
					run page, and it is the evidence the headline rests on: throughput flattening while
					latency turns upward is what "peak" means, drawn.
				</p>
			</section>
		</div>
	</Section>

	<Section title="the three ways a target can have no peak">
		<div class="measure text-prose text-foreground-secondary space-y-5">
			<p>
				A capacity measurement can fail to produce a number, and when it does this site says so in
				words. Nothing is substituted: not a zero, not the top of the ramp, and never the paced
				number wearing the other one's name.
			</p>
			<dl class="space-y-4">
				<div>
					<dt class="text-foreground font-medium">A peak, at a stated objective</dt>
					<dd class="mt-1">
						The ramp went far enough to break the target: its last step breached the objective. The
						number reported is the <em>fastest</em> step that did hold it — ties going to the lower concurrency,
						since the same throughput for less concurrency is the better result — with the objective always
						beside it ("12.5k req/s at p99 &lt; 50 ms") and with the concurrency it was reached at. This
						is the only outcome that produces a comparable number, and the only one given a rank when
						the table is sorted by peak throughput.
					</dd>
				</div>
				<div>
					<dt class="text-foreground font-medium">"at least N req/s — knee not reached"</dt>
					<dd class="mt-1">
						The ramp's last step still held the objective, so it stopped before the target did. The
						best qualifying throughput is a <em>lower bound</em>, not a peak: this target sustains
						at least that much and may sustain considerably more. Note that this does not require
						throughput to still be climbing — a curve can flatten, or even dip after the connection
						pool saturates, and still end without breaching. A visible bend is not the same finding
						as a measured limit. It is shown with "at least", drawn faint, given no rank, and sorted
						below every measured peak, because a ramp that ended early is not evidence of beating a
						target that was measured to its limit. It is also a finding about the workload, and it
						is visible so the ramp gets extended.
					</dd>
				</div>
				<div>
					<dt class="text-foreground font-medium">"never met the p99 target"</dt>
					<dd class="mt-1">
						Even the smallest step breached the objective. There is no peak and no number is
						reported in place of one. The curve is still drawn, because how far over the objective
						the first step landed is the useful part.
					</dd>
				</div>
				<div>
					<dt class="text-foreground font-medium">"not measured"</dt>
					<dd class="mt-1">
						The run predates the saturation suite, or did not run it for that target. Runs published
						before the suite existed carry an older field also called "saturation" — a knee
						heuristic computed off the <em>paced</em> run, which produced a number whether or not it found
						a knee. This dashboard does not read it. A target with no saturation measurement says "not
						measured", which is the true statement.
					</dd>
				</div>
			</dl>
		</div>
	</Section>

	<Section title="fair means two different things">
		<div class="measure text-prose text-foreground-secondary space-y-7">
			<p>
				Fairness on this site has two meanings, and they pull in opposite directions. Keeping them
				apart is what makes the tables readable; blurring them is the easiest way to mislead.
			</p>

			<section>
				<h3 class="text-heading text-foreground mb-2 font-semibold">
					Inside a comparison group — identical, and enforced.
				</h3>
				<p>
					A <em>comparison group</em> is the set of targets claiming to be directly comparable. Every
					target in one runs the same worker count, the same connection pool size and the same server
					tuning, which is what makes the gap between two of its rows attributable to the library rather
					than to the setup. It is a hard requirement in the runner: a group whose targets declare different
					harnesses fails the run rather than publishing a quietly unequal comparison. The "vs …" figure
					inside a ranking row is scoped to exactly this — the drizzle target in that same group, under
					that same harness.
				</p>
				<p class="mt-3">
					A group is usually a database, but it is not the same thing, and it splits wherever the
					harness genuinely cannot be equalised. <code class="text-meta font-mono">bun:sqlite</code>
					is synchronous on a single-threaded runtime, so giving it the Rust stack's pool of eight would
					be fiction rather than fairness. It therefore sits in its own SQLite group with drizzle-orm
					— same runtime, same pool of one, same pragmas, a real library comparison — while the Rust stack
					keeps its own. Both are still SQLite, and both still appear in the one table with SQLite in
					the database column: the split changes what a row is
					<em>measured against</em>, never whether it is shown.
				</p>
			</section>

			<section>
				<h3 class="text-heading text-foreground mb-2 font-semibold">
					Across groups — different, and declared.
				</h3>
				<p>
					An embedded engine and a client/server engine are not made comparable by forcing them into
					one configuration; they are made <em>equally crippled</em>. So across groups the harness
					is deliberately allowed to differ, each stack running in the shape it is actually deployed
					in. That difference is part of what the comparison shows — which means it has to be
					visible, or a reader will read a stack difference as a library difference.
				</p>
				<p class="mt-3">
					Each run records its harness per group, and the ranking prints them as a strip above the
					table: one line per group, giving workers, pool size, tuning, and whether within-group
					identity was verified. Two rows in one group share that whole line; two rows in different
					groups share none of it. A group whose run declared no harness says "harness not declared"
					rather than borrowing a neighbour's, and a group that verified identity while exempting
					some targets says how many were exempted.
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
