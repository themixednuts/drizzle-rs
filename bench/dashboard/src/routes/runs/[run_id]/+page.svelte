<script lang="ts">
	import { loadTimeseries } from '$lib/api.remote';
	import LatencyBars from '$lib/components/LatencyBars.svelte';
	import QueryMetricBars from '$lib/components/QueryMetricBars.svelte';
	import SparkLine from '$lib/components/SparkLine.svelte';
	import {
		fmtCpu,
		fmtDate,
		fmtDuration,
		fmtGb,
		fmtLatency,
		fmtPct,
		fmtRps,
		shortHash,
		suiteLabel
	} from '$lib/format';
	import { RunDetailState } from './run-detail.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunDetailState(() => data);
	let primary = $derived(view.primarySummary);
</script>

<svelte:head>
	<title>{view.runName} - drizzle-rs/bench</title>
</svelte:head>

<main class="wrap">
	<div class="ph">
		<div>
			<div class="ph-l">/ runs / detail</div>
			<h1 class="ph-h">{view.runName}</h1>
			<div class="ph-sub">
				<span class="mono">{view.manifest.run_id}</span> / cohort <span class="mono">{view.manifest.cohort_id}</span> / {suiteLabel(view.manifest.suite)} / {shortHash(view.manifest.git)} / {fmtDate(view.manifest.start)}
			</div>
		</div>
		<div class="ph-sub">
			<span class="badge badge--{view.manifest.status}">{view.manifest.status}</span>
			<span class="mu"> / </span>
			<a class="acc" href="/runs">all runs</a>
		</div>
	</div>

	{#if primary}
		{@const p = primary.primary}
		<div class="kpi">
			<div class="k">
				<div class="k-l">rps</div>
				<div class="k-v">{fmtRps(p.rps.avg)}</div>
				<div class="k-d">peak {fmtRps(p.rps.peak)}</div>
			</div>
			<div class="k">
				<div class="k-l">avg</div>
				<div class="k-v">{fmtLatency(p.latency.avg)}</div>
				<div class="k-d">latency</div>
			</div>
			<div class="k">
				<div class="k-l">lat p95</div>
				<div class="k-v">{fmtLatency(p.latency.p95)}</div>
				<div class="k-d">latency</div>
			</div>
			<div class="k">
				<div class="k-l">lat p99</div>
				<div class="k-v">{fmtLatency(p.latency.p99)}</div>
				<div class="k-d">latency</div>
			</div>
			<div class="k">
				<div class="k-l">cpu</div>
				<div class="k-v">{fmtCpu(p.cpu.avg)}</div>
				<div class="k-d">peak {fmtCpu(p.cpu.peak)}</div>
			</div>
			<div class="k">
				<div class="k-l">{p.mem ? 'mem' : 'err'}</div>
				<div class="k-v">{p.mem ? `${p.mem.avg.toFixed(1)}MB` : fmtPct(p.err)}</div>
				<div class="k-d">{p.mem ? `peak ${p.mem.peak.toFixed(1)}MB` : 'error rate'}</div>
			</div>
		</div>
	{/if}

	<section class="sec">
		<div class="sec-h"><span>run metadata</span></div>
		<div class="table-scroll">
			<table class="t meta-table">
				<tbody>
					<tr><td class="mu">suite</td><td>{suiteLabel(view.manifest.suite)}</td><td class="mu">workload</td><td>{view.manifest.workload}</td></tr>
					<tr><td class="mu">commit</td><td>{view.manifest.git}</td><td class="mu">duration</td><td>{fmtDuration(view.manifest.start, view.manifest.end)}</td></tr>
					<tr><td class="mu">runner</td><td>{view.manifest.runner.class} / {view.manifest.runner.os}</td><td class="mu">hardware</td><td>{view.manifest.runner.cpu} / {view.manifest.runner.cores}c / {fmtGb(view.manifest.runner.mem_gb)}</td></tr>
					<tr><td class="mu">trials</td><td>{view.manifest.trials.count} / {view.manifest.trials.aggregate}</td><td class="mu">seed</td><td>{view.manifest.seed}</td></tr>
					<tr><td class="mu">metrics</td><td>{view.manifest.runner.metrics.cpu_scope} cpu / {view.manifest.runner.metrics.memory_scope} memory</td><td class="mu">network</td><td>{view.manifest.runner.metrics.network_scope}</td></tr>
					<tr><td class="mu">headroom</td><td>cpu {fmtCpu(view.manifest.runner.headroom.cpu_peak)} / net {view.manifest.runner.headroom.net_peak == null ? 'unmeasured' : fmtCpu(view.manifest.runner.headroom.net_peak)}</td><td class="mu">targets</td><td>{view.manifest.targets.length}</td></tr>
				</tbody>
			</table>
		</div>
	</section>

	<section class="sec">
		<div class="sec-h"><span>load and dataset</span></div>
		<div class="table-scroll">
			<table class="t meta-table">
				<tbody>
					<tr><td class="mu">executor</td><td>{view.manifest.load.executor}</td><td class="mu">stages</td><td>{view.manifest.load.stages}</td></tr>
					<tr><td class="mu">load duration</td><td>{view.manifest.load.duration_s}s</td><td class="mu">max vus</td><td>{view.manifest.load.max_vus.toLocaleString()}</td></tr>
					<tr><td class="mu">requests</td><td>{view.manifest.load.requests.toLocaleString()}</td><td class="mu">pacing</td><td>{view.manifest.load.pacing}</td></tr>
					<tr><td class="mu">orders</td><td>{view.manifest.dataset.orders.toLocaleString()}</td><td class="mu">customers</td><td>{view.manifest.dataset.customers.toLocaleString()}</td></tr>
					<tr><td class="mu">products</td><td>{view.manifest.dataset.products.toLocaleString()}</td><td class="mu">suppliers</td><td>{view.manifest.dataset.suppliers.toLocaleString()}</td></tr>
					<tr><td class="mu">details/order</td><td>{view.manifest.dataset.details_per_order}</td><td class="mu">seed</td><td>{view.manifest.seed}</td></tr>
				</tbody>
			</table>
		</div>
	</section>

	{#if view.queries.length > 0}
		<section class="sec">
			<div class="sec-h">
				<span>request mix</span>
				<span class="mu">{view.totalQueryMix.toLocaleString()} materialized requests</span>
			</div>
			<div class="workload-lens">
				<div class="workload-note">
					This is workload composition only: the generated HTTP routes and how often each route appears in the request list.
					Metric graphs are per target below, where route-level RPS and latency come from measured request samples.
				</div>
				<div class="query-bars">
					{#each view.queries as query}
						<div class="query-bar-row">
							<div>
								<div class="query-bar-title">{query.name}</div>
								<div class="mu mono">{query.method} {query.path}</div>
							</div>
							<div class="query-share">{fmtPct(view.queryShare(query))} / {query.mix.toLocaleString()}</div>
						</div>
					{/each}
				</div>
			</div>
		</section>

		<section class="sec">
			<div class="sec-h">
				<span>query catalog</span>
				<span class="mu">{view.queries.length} operations / SQL shapes</span>
			</div>
			<div class="query-list">
				{#each view.queries as query}
					<details class="query-card">
						<summary>
							<span>{query.name}</span>
							<span class="mu mono">{query.method} {query.path} / {query.mix.toLocaleString()}</span>
						</summary>
						<div class="query-meta">
							<span class="mu">params</span>
							<span>{query.params.length ? query.params.join(', ') : 'none'}</span>
						</div>
						{#each query.sql as shape}
							<pre class="sql"><code>{shape.text}</code></pre>
						{/each}
					</details>
				{/each}
			</div>
		</section>
	{/if}

	<section class="sec">
		<div class="sec-h"><span>target summary</span></div>
		<div class="table-scroll">
			<table class="t">
				<thead>
					<tr>
						<th>target</th>
						<th>group</th>
						<th class="n">rps</th>
						<th class="n">peak</th>
						<th class="n">lat avg</th>
						<th class="n">lat p95</th>
						<th class="n">lat p99</th>
						<th class="n">cpu</th>
						<th class="n">err</th>
						<th style="width: 160px">throughput</th>
					</tr>
				</thead>
				<tbody>
					{#each view.sortedSummaries as summary}
						{@const p = summary.primary}
						{@const display = view.targetDisplay(summary.target_id)}
						<tr class={view.rowClass(summary)}>
							<td>
								<span class="target-link">{display.name}</span>
								<span class="target-badges" title={summary.target_id}>
									{#each display.badges as badge, index}
										{#if index > 0}<span class="target-slash">/</span>{/if}
										<span>{badge}</span>
									{/each}
								</span>
							</td>
							<td class="mu">{view.targetGroup(summary)}</td>
							<td class="n">{fmtRps(p.rps.avg)}</td>
							<td class="n fade">{fmtRps(p.rps.peak)}</td>
							<td class="n fade">{fmtLatency(p.latency.avg)}</td>
							<td class="n">{fmtLatency(p.latency.p95)}</td>
							<td class="n fade">{fmtLatency(p.latency.p99)}</td>
							<td class="n mu">{fmtCpu(p.cpu.avg)}</td>
							<td class="n mu">{fmtPct(p.err)}</td>
							<td><span class="bar" class:acc={summary === primary} style={view.barStyle(summary)}></span></td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</section>

	{#each view.groups as [groupName, groupItems]}
		<section class="sec">
			<div class="sec-h">
				<span>{groupName} detail</span>
				<span class="mu">{groupItems.length} target{groupItems.length === 1 ? '' : 's'}</span>
			</div>
			<div class="target-detail">
				{#each groupItems as summary}
					{@const p = summary.primary}
					{@const meta = view.targetMeta(summary.target_id)}
					{@const display = view.targetDisplay(summary.target_id)}
					<article class="target-row">
						<div class="target-head">
							<h2>
								{display.name}
								<span class="target-badges" title={summary.target_id}>
									{#each display.badges as badge, index}
										{#if index > 0}<span class="target-slash">/</span>{/if}
										<span>{badge}</span>
									{/each}
								</span>
							</h2>
							<span class="badge badge--{p.err > 0 ? 'failed' : 'success'}">{fmtPct(p.err)} err</span>
						</div>
						{#if view.targetDescription(summary.target_id)}
							<p class="target-desc">{view.targetDescription(summary.target_id)}</p>
						{/if}

						<div class="mini-grid">
							<div><span class="mu">rps</span><strong>{fmtRps(p.rps.avg)}</strong></div>
							<div><span class="mu">p95</span><strong>{fmtLatency(p.latency.p95)}</strong></div>
							<div><span class="mu">p99</span><strong>{fmtLatency(p.latency.p99)}</strong></div>
							<div><span class="mu">cpu</span><strong>{fmtCpu(p.cpu.avg)}</strong></div>
						</div>

						<div class="detail-split">
							<div>
								<div class="metric-label">latency distribution</div>
								<LatencyBars latency={p.latency} />
							</div>
							<div>
								<div class="sparkline-tabs">
									<button class="pill" class:on={view.selectedMetric === 'rps'} onclick={() => view.selectMetric('rps')}>rps</button>
									<button class="pill" class:on={view.selectedMetric === 'latency'} onclick={() => view.selectMetric('latency')}>p95</button>
									<button class="pill" class:on={view.selectedMetric === 'cpu'} onclick={() => view.selectMetric('cpu')}>cpu</button>
									{#if p.mem}
										<button class="pill" class:on={view.selectedMetric === 'mem'} onclick={() => view.selectMetric('mem')}>mem</button>
									{/if}
								</div>
								<div class="spark-help">{view.selectedMetricHelp}</div>
								<svelte:boundary>
									{#snippet pending()}
										<div class="skeleton" style="height: 48px"></div>
									{/snippet}

									{@const ts = await loadTimeseries({ runId: view.manifest.run_id, targetId: summary.target_id })}
									{#if ts}
										<SparkLine points={ts.points} metric={view.selectedMetric} />
										<QueryMetricBars queries={view.queries} points={ts.points} metric={view.selectedMetric} />
									{:else}
										<div class="spark-empty">no timeseries data</div>
									{/if}
								</svelte:boundary>
							</div>
						</div>

						<div class="table-scroll">
							<table class="t">
								<tbody>
									{#if meta}
										<tr><td class="mu">runtime</td><td>{meta.runtime.name} {meta.runtime.ver}</td><td class="mu">orm</td><td>{meta.orm.name} {meta.orm.ver}</td></tr>
										<tr><td class="mu">driver</td><td>{meta.driver.name} {meta.driver.ver}</td><td class="mu">wire</td><td>{meta.wire.format}</td></tr>
										<tr><td class="mu">workers / pool</td><td>{meta.proc.workers} / {meta.pool.max}</td><td class="mu">fair contract</td><td>{meta.fair.contract} / {meta.contract.ver}</td></tr>
									{/if}
									<tr><td class="mu">spread rps</td><td>{fmtRps(summary.spread.rps.min)} - {fmtRps(summary.spread.rps.max)}</td><td class="mu">spread p95</td><td>{fmtLatency(summary.spread.p95.min)} - {fmtLatency(summary.spread.p95.max)}</td></tr>
									<tr><td class="mu">saturation rps</td><td>{fmtRps(summary.saturation.knee_rps)}</td><td class="mu">saturation p95</td><td>{fmtLatency(summary.saturation.knee_p95)}</td></tr>
								</tbody>
							</table>
						</div>
					</article>
				{/each}
			</div>
		</section>
	{/each}
</main>

<style>
	.meta-table td:nth-child(odd) {
		width: 140px;
	}

	.target-detail {
		display: grid;
		gap: 18px;
	}

	.target-row {
		padding-bottom: 18px;
		border-bottom: 1px solid var(--rule-soft);
	}

	.query-list {
		display: grid;
		gap: 10px;
		min-width: 0;
	}

	.workload-lens {
		display: grid;
		grid-template-columns: minmax(180px, 0.7fr) minmax(260px, 1.3fr);
		gap: 24px;
		align-items: start;
	}

	.workload-note {
		border-left: 2px solid var(--acc);
		padding-left: 12px;
		color: var(--ink-3);
		font-family: var(--font-mono);
		font-size: 11.5px;
		line-height: 1.6;
	}

	.query-bars {
		display: grid;
		gap: 9px;
	}

	.query-bar-row {
		display: grid;
		grid-template-columns: minmax(160px, 1fr) auto;
		align-items: center;
		gap: 12px;
		font-family: var(--font-mono);
		font-size: 11.5px;
	}

	.query-bar-title {
		color: var(--ink);
	}

	.query-share {
		color: var(--ink-3);
		white-space: nowrap;
	}

	.query-card {
		border: 1px solid var(--rule-soft);
		background: color-mix(in srgb, var(--bg-2) 55%, transparent);
		padding: 12px 14px;
		min-width: 0;
		max-width: 100%;
		overflow: clip;
	}

	.query-card summary {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 16px;
		cursor: pointer;
		min-width: 0;
	}

	.query-card summary span:first-child {
		min-width: 0;
	}

	.query-card summary .mono {
		min-width: 0;
		overflow-wrap: anywhere;
		text-align: right;
	}

	.query-meta {
		display: grid;
		grid-template-columns: 90px minmax(0, 1fr);
		gap: 12px;
		margin-top: 12px;
		font-family: var(--font-mono);
		font-size: 12px;
		min-width: 0;
	}

	.query-meta span:last-child {
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.sql {
		margin: 10px 0 0;
		padding: 10px;
		overflow-x: auto;
		max-width: 100%;
		background: var(--bg);
		border: 1px solid var(--rule-soft);
		color: var(--ink-2);
		font-family: var(--font-mono);
		font-size: 11.5px;
		line-height: 1.45;
	}

	.sql code {
		display: block;
		min-width: 0;
		width: max-content;
		max-width: none;
	}

	.target-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 10px;
	}

	.target-head h2 {
		color: var(--ink);
		font-size: 14px;
		font-weight: 500;
	}

	.target-desc {
		margin: -3px 0 12px;
		color: var(--ink-3);
		font-size: 12px;
	}

	.mini-grid {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 12px;
		margin-bottom: 14px;
		font-family: var(--font-mono);
	}

	.mini-grid div {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.mini-grid strong {
		font-size: 16px;
		font-weight: 500;
	}

	.detail-split {
		display: grid;
		grid-template-columns: minmax(220px, 0.9fr) minmax(260px, 1.1fr);
		gap: 24px;
		margin-bottom: 12px;
	}

	.sparkline-tabs {
		display: flex;
		gap: 0;
		margin-bottom: 8px;
		font-family: var(--font-mono);
	}

	.spark-help {
		margin: -2px 0 6px;
		color: var(--ink-3);
		font-family: var(--font-mono);
		font-size: 10.5px;
		line-height: 1.45;
	}

	.spark-empty {
		height: 48px;
		display: flex;
		align-items: center;
		color: var(--ink-3);
		font-family: var(--font-mono);
		font-size: 11.5px;
	}

	@media (max-width: 760px) {
		.mini-grid,
		.detail-split,
		.workload-lens,
		.query-bar-row {
			grid-template-columns: 1fr;
		}
	}
</style>
