<script lang="ts">
	import { fmtDate, fmtDuration, shortHash } from '$lib/format';
	import { RunsPageState } from '../home.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new RunsPageState(() => data, '/runs');
</script>

<svelte:head>
	<title>runs - drizzle-rs/bench</title>
</svelte:head>

<main class="wrap">
	<div class="ph">
		<div>
			<div class="ph-l">/ runs</div>
			<h1 class="ph-h">all runs</h1>
			<div class="ph-sub">{view.totalRuns} runs / {view.suites.length} suites / {view.totalTargets} targets</div>
		</div>
		<div class="ph-sub">
			<a class="acc" href="/api/v1/runs/latest?suite={view.suite ?? view.suites[0] ?? ''}">latest json</a>
		</div>
	</div>

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
		<input
			class="search"
			type="search"
			placeholder="run, commit, target..."
			value={view.query}
			oninput={view.search}
		/>
	</div>

	<section class="sec">
		<div class="sec-h">
			<span>{view.filteredRuns.length} matching runs</span>
		</div>
		{#if view.filteredRuns.length === 0}
			<div class="empty">
				<p class="empty-text">No runs match the current filters.</p>
			</div>
		{:else}
			<div class="table-scroll">
				<table class="t">
					<thead>
						<tr>
							<th>run</th>
							<th>suite</th>
							<th>status</th>
							<th>class</th>
							<th>commit</th>
							<th class="n">targets</th>
							<th class="n">duration</th>
							<th class="n">started</th>
						</tr>
					</thead>
					<tbody>
						{#each view.filteredRuns as run}
							<tr>
								<td><a class="acc" href="/runs/{run.run_id}">{run.run_id}</a></td>
								<td class="mu">{run.suite}</td>
								<td><span class="badge badge--{run.status}">{run.status}</span></td>
								<td class="mu">{run.class}</td>
								<td>{shortHash(run.git)}</td>
								<td class="n">{run.targets.length}</td>
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
