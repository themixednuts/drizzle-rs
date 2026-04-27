<script lang="ts">
	import { fmtCpu, fmtDate, fmtDuration, fmtLatency, fmtPct, fmtRps, shortHash } from '$lib/format';
	import { RunsPageState } from './home.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunsPageState(() => data);
	let latest = $derived(view.latest);
</script>

<svelte:head>
	<title>drizzle-rs/bench</title>
</svelte:head>

<main class="wrap">
	<div class="ph">
		<div>
			<div class="ph-l">/ overview</div>
			<h1 class="ph-h">drizzle-rs/bench</h1>
			<div class="ph-sub">{view.overviewMeta}</div>
		</div>
		{#if latest}
			<div class="ph-sub">
				last run <a class="acc" href="/runs/{latest.run.run_id}">{shortHash(latest.run.git)}</a>
				/ {fmtDate(latest.run.start)}
			</div>
		{/if}
	</div>

	{#if view.ours}
		<div class="kpi">
			{#each view.kpis as item}
				<div class="k">
					<div class="k-l">{item.label}</div>
					<div class="k-v">{item.value}</div>
					<div class="k-d">{item.detail}</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="filt">
		<span class="filt-l">suite</span>
		<div class="filt-pills">
			<a href={view.buildUrl(null, view.status)} class="pill" class:on={!view.suite}>all</a>
			{#each view.suites as suite}
				<a href={view.buildUrl(suite, view.status)} class="pill" class:on={view.suite === suite}>{suite}</a>
			{/each}
		</div>
		<span class="filt-l">status</span>
		<div class="filt-pills">
			<a href={view.buildUrl(view.suite, null)} class="pill" class:on={!view.status}>all</a>
			{#each view.statuses as status}
				<a href={view.buildUrl(view.suite, status)} class="pill" class:on={view.status === status}>{status}</a>
			{/each}
		</div>
		<span class="spacer"></span>
		<span class="filt-l">{view.filterMeta}</span>
	</div>

	<section class="sec">
		<div class="sec-h">
			<span>latest leaderboard</span>
			{#if latest}<a href="/runs/{latest.run.run_id}">run detail</a>{/if}
		</div>
		{#if view.leaderboard.length === 0}
			<div class="empty">
				<p class="empty-text">No successful run summaries are available.</p>
			</div>
		{:else}
			<div class="table-scroll">
				<table class="t">
					<thead>
						<tr>
							<th class="n" style="width: 24px">#</th>
							<th>target</th>
							<th class="n">rps</th>
							<th class="n">avg</th>
							<th class="n">p95</th>
							<th class="n">p99</th>
							<th class="n">cpu</th>
							<th class="n">err</th>
							<th style="width: 160px">throughput</th>
							<th class="n">vs ours</th>
						</tr>
					</thead>
					<tbody>
						{#each view.leaderboard as summary, i}
							{@const p = summary.primary}
							<tr class={view.rowClass(summary)}>
								<td class="n mu">{String(i + 1).padStart(2, '0')}</td>
								<td>
									<a href="/runs/{summary.run_id}" class="acc">{summary.target_id}</a>
									{#if summary.group}<span class="mu"> / {summary.group}</span>{/if}
								</td>
								<td class="n">{fmtRps(p.rps.avg)}</td>
								<td class="n fade">{fmtLatency(p.latency.avg)}</td>
								<td class="n">{fmtLatency(p.latency.p95)}</td>
								<td class="n fade">{fmtLatency(p.latency.p99)}</td>
								<td class="n mu">{fmtCpu(p.cpu.avg)}</td>
								<td class="n mu">{fmtPct(p.err)}</td>
								<td><span class="bar" class:acc={summary === view.ours} style={view.barStyle(summary)}></span></td>
								<td class="n {view.deltaClass(summary)}">{view.deltaText(summary)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>

	<section class="sec">
		<div class="sec-h">
			<span>recent runs</span>
			<a href="/runs">all {view.totalRuns}</a>
		</div>
		{#if view.runs.length === 0}
			<div class="empty">
				<p class="empty-text">No runs match the selected filters.</p>
				<p class="empty-sub">Try changing suite or status.</p>
			</div>
		{:else}
			<div class="table-scroll">
				<table class="t">
					<thead>
						<tr>
							<th>run</th>
							<th>suite</th>
							<th>status</th>
							<th class="n">targets</th>
							<th>commit</th>
							<th class="n">duration</th>
							<th class="n">started</th>
						</tr>
					</thead>
					<tbody>
						{#each view.recentRuns as run}
							<tr>
								<td><a class="acc" href="/runs/{run.run_id}">{run.run_id}</a></td>
								<td class="mu">{run.suite}</td>
								<td><span class="badge badge--{run.status}">{run.status}</span></td>
								<td class="n">{run.targets.length}</td>
								<td class="mu">{shortHash(run.git)}</td>
								<td class="n mu">{fmtDuration(run.start, run.end)}</td>
								<td class="n mu">{fmtDate(run.start)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>
</main>
