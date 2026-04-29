<script lang="ts">
	import TrendChart from '$lib/components/TrendChart.svelte';
	import { fmtCpu, fmtLatency, fmtPct, fmtRps, shortHash, suiteLabel } from '$lib/format';
	import { TrendsPageState } from './trends.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new TrendsPageState(() => data);
</script>

<svelte:head>
	<title>trends - drizzle-rs/bench</title>
</svelte:head>

<main class="wrap">
	<div class="ph">
		<div>
			<div class="ph-l">/ trends</div>
			<h1 class="ph-h">performance trends</h1>
			<div class="ph-sub">{view.trends.length} sets{view.targetLabel ? ` / ${view.targetLabel}` : ''}</div>
		</div>
		{#if view.latest}
			<div class="ph-sub">latest <a class="acc" href="/runs/{view.latest.run_id}">{shortHash(view.latest.git)}</a></div>
		{/if}
	</div>

	<div class="filt">
		<span class="filt-l">suite</span>
		<div class="filt-pills">
			<a href={view.buildUrl(null, view.targetKey)} class="pill" class:on={!view.suite}>all</a>
			{#each view.suites as suite}
				<a href={view.buildUrl(suite, view.targetKey)} class="pill" class:on={view.suite === suite}>{suiteLabel(suite)}</a>
			{/each}
		</div>
		<span class="filt-l">target</span>
		<select class="sel" value={view.targetKey ?? ''} onchange={view.selectTarget}>
			<option value="">select target...</option>
			{#each view.targets as target}
				<option value={target.key}>{target.label}</option>
			{/each}
		</select>
	</div>

	{#if !view.targetKey}
		<div class="empty">
			<p class="empty-text">Select a target to view trend data.</p>
		</div>
	{:else if view.trends.length === 0}
		<div class="empty">
			<p class="empty-text">No successful trend data for {view.targetLabel}.</p>
			<p class="empty-sub">Runs with this target will appear after CI publishes summaries.</p>
		</div>
	{:else}
		{@const latest = view.latest}
		{@const prev = view.previous}
		{#if latest}
			<div class="kpi">
				<div class="k">
					<div class="k-l">rps</div>
					<div class="k-v">{fmtRps(latest.rps_avg)}</div>
					{#if prev}
						{@const d = (latest.rps_avg - prev.rps_avg) / (prev.rps_avg || 1)}
						<div class="k-d" class:up={d > 0.005} class:down={d < -0.005} class:flat={Math.abs(d) <= 0.005}>{d >= 0 ? '+' : ''}{(d * 100).toFixed(1)}%</div>
					{/if}
				</div>
				<div class="k">
					<div class="k-l">peak</div>
					<div class="k-v">{fmtRps(latest.rps_peak)}</div>
					<div class="k-d">rps</div>
				</div>
				<div class="k">
					<div class="k-l">lat p95</div>
					<div class="k-v">{fmtLatency(latest.latency_p95)}</div>
					{#if prev}
						{@const d = (latest.latency_p95 - prev.latency_p95) / (prev.latency_p95 || 1)}
						<div class="k-d" class:up={d < -0.005} class:down={d > 0.005} class:flat={Math.abs(d) <= 0.005}>{d >= 0 ? '+' : ''}{(d * 100).toFixed(1)}%</div>
					{/if}
				</div>
				<div class="k">
					<div class="k-l">lat p99</div>
					<div class="k-v">{fmtLatency(latest.latency_p99)}</div>
					<div class="k-d">latency</div>
				</div>
				<div class="k">
					<div class="k-l">cpu</div>
					<div class="k-v">{fmtCpu(latest.cpu_avg)}</div>
					<div class="k-d">average</div>
				</div>
				<div class="k">
					<div class="k-l">err</div>
					<div class="k-v">{fmtPct(latest.err)}</div>
					<div class="k-d">rate</div>
				</div>
				{#if latest.mem_avg !== undefined}
					<div class="k">
						<div class="k-l">mem</div>
						<div class="k-v">{latest.mem_avg.toFixed(1)}MB</div>
						<div class="k-d">{latest.mem_peak?.toFixed(1) ?? latest.mem_avg.toFixed(1)}MB peak</div>
					</div>
				{/if}
			</div>
		{/if}

		<section class="sec">
			<div class="sec-h"><span>charts</span></div>
			<TrendChart points={view.trends} metric="rps_avg" label="rps avg" color="var(--acc)" formatValue={fmtRps} />
			<TrendChart points={view.trends} metric="latency_p95" label="p95 latency" color="var(--ink-2)" formatValue={fmtLatency} />
			<TrendChart points={view.trends} metric="latency_p99" label="p99 latency" color="var(--ink-2)" formatValue={fmtLatency} />
			<TrendChart points={view.trends} metric="cpu_avg" label="cpu avg" color="var(--ink-2)" formatValue={fmtCpu} />
			{#if latest?.mem_avg !== undefined}
				<TrendChart points={view.trends} metric="mem_avg" label="memory avg" color="var(--ink-2)" formatValue={(n) => `${n.toFixed(1)}MB`} />
			{/if}
		</section>

		<section class="sec">
			<div class="sec-h"><span>run history</span></div>
			<div class="table-scroll">
				<table class="t">
					<thead>
						<tr>
							<th>run</th>
							<th>commit</th>
							<th class="n">rps avg</th>
							<th class="n">rps peak</th>
							<th class="n">lat p95</th>
							<th class="n">lat p99</th>
							<th class="n">cpu</th>
							<th class="n">err</th>
						</tr>
					</thead>
					<tbody>
						{#each view.reversedTrends as point}
							<tr>
								<td><a class="acc" href="/runs/{point.run_id}">{point.run_id}</a></td>
								<td class="mu">{shortHash(point.git)}</td>
								<td class="n">{fmtRps(point.rps_avg)}</td>
								<td class="n fade">{fmtRps(point.rps_peak)}</td>
								<td class="n">{fmtLatency(point.latency_p95)}</td>
								<td class="n fade">{fmtLatency(point.latency_p99)}</td>
								<td class="n mu">{fmtCpu(point.cpu_avg)}</td>
								<td class="n mu">{fmtPct(point.err)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{/if}
</main>
