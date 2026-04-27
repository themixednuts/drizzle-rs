<script lang="ts">
	import { compareMetricOptions } from '$lib/compare';
	import { compareRuns } from '$lib/compare-form.remote';
	import { fmtDelta } from '$lib/format';
	import { ComparePageState } from './compare.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new ComparePageState(() => data);
</script>

<svelte:head>
	<title>compare - drizzle-rs/bench</title>
</svelte:head>

<main class="wrap">
	<div class="ph">
		<div>
			<div class="ph-l">/ compare</div>
			<h1 class="ph-h">compare runs</h1>
			<div class="ph-sub">diff one metric across common targets</div>
		</div>
		<div class="ph-sub">{view.runs.length} runs available</div>
	</div>

	<form class="filt compare-form" {...compareRuns}>
		<label class="filt-l" for="base">base</label>
		<select name="base" id="base" class="sel" value={view.base ?? ''}>
			<option value="">select base...</option>
			{#each view.runs as run}
				<option value={run.run_id} selected={run.run_id === view.base}>{run.run_id} ({run.suite})</option>
			{/each}
		</select>

		<label class="filt-l" for="head">head</label>
		<select name="head" id="head" class="sel" value={view.head ?? ''}>
			<option value="">select head...</option>
			{#each view.runs as run}
				<option value={run.run_id} selected={run.run_id === view.head}>{run.run_id} ({run.suite})</option>
			{/each}
		</select>

		<label class="filt-l" for="metric">metric</label>
		<select name="metric" id="metric" class="sel">
			{#each compareMetricOptions as metric}
				<option value={metric.value} selected={metric.value === view.metric}>{metric.label}</option>
			{/each}
		</select>

		<span class="spacer"></span>
		<button type="submit" class="pill on">compare</button>
	</form>

	{#if view.items}
		<section class="sec">
			<div class="sec-h">
				<span>{view.metric} diff</span>
				<span class="mu">{view.items.length} common targets</span>
			</div>
			{#if view.items.length === 0}
				<div class="empty">
					<p class="empty-text">No comparable targets found.</p>
				</div>
			{:else}
				<div class="table-scroll">
					<table class="t">
						<thead>
							<tr>
								<th>target</th>
								<th class="n">base</th>
								<th class="n">head</th>
								<th class="n">delta</th>
								<th class="n">pct</th>
								<th style="width: 180px">change</th>
							</tr>
						</thead>
						<tbody>
							{#each view.items as item}
								{@const cls = view.deltaClass(item.delta_pct)}
								<tr>
									<td>{item.target_id}</td>
									<td class="n">{view.formatMetricValue(item.base_value)}</td>
									<td class="n">{view.formatMetricValue(item.head_value)}</td>
									<td class="n {cls}">{view.formatMetricValue(item.delta)}</td>
									<td class="n {cls}">{fmtDelta(item.delta_pct)}</td>
									<td>
										<div class="diff-track">
											<span class="diff-center"></span>
											<span class="diff-bar {cls}" style={view.barStyle(item.delta_pct)}></span>
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>
	{:else if view.base || view.head}
		<div class="empty">
			<p class="empty-text">Select both a base and head run.</p>
		</div>
	{/if}
</main>

<style>
	.compare-form {
		display: grid;
		grid-template-columns:
			auto minmax(180px, 1fr)
			auto minmax(180px, 1fr)
			auto minmax(150px, 0.65fr)
			auto;
		align-items: center;
	}

	.compare-form .sel {
		width: 100%;
		min-width: 0;
	}

	.compare-form .spacer {
		display: none;
	}

	.diff-track {
		position: relative;
		width: 100%;
		height: 14px;
		background: var(--bg-2);
	}

	.diff-center {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 50%;
		width: 1px;
		background: var(--rule);
	}

	.diff-bar {
		position: absolute;
		top: 4px;
		bottom: 4px;
		background: var(--ink-4);
	}

	.diff-bar.delta-positive {
		background: var(--pos);
	}

	.diff-bar.delta-negative {
		background: var(--neg);
	}

	@media (max-width: 760px) {
		.compare-form {
			display: flex;
			align-items: stretch;
			flex-direction: column;
		}
	}
</style>
