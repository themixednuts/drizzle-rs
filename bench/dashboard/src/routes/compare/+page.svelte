<script lang="ts">
	import { compareMetricOptions } from '$lib/compare';
	import { compareTargets } from '$lib/compare-form.remote';
	import {
		fmtDate,
		fmtDelta,
		fmtDuration,
		fmtPct,
		runDisplayName,
		shortHash,
		suiteLabel
	} from '$lib/format';
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
			<h1 class="ph-h">compare targets</h1>
			<div class="ph-sub">rank every target result in one benchmark set</div>
		</div>
		<div class="ph-sub">{view.cohorts.length} benchmark sets available</div>
	</div>

	<form class="filt compare-form" {...compareTargets}>
		<label class="filt-l" for="cohort">set</label>
		<select name="cohort" id="cohort" class="sel" value={view.cohortId ?? ''} onchange={view.updateComparison}>
			{#each view.cohorts as cohort}
				<option value={cohort.id} selected={cohort.id === view.cohortId}>
					{runDisplayName(cohort)} / {shortHash(cohort.git)} / {fmtDate(cohort.start)} / {cohort.result_count} results
				</option>
			{/each}
		</select>

		<label class="filt-l" for="baseline">baseline</label>
		<select name="baseline" id="baseline" class="sel" value={view.baseline ?? ''} onchange={view.updateComparison}>
			{#each view.targets as target}
				<option value={target.key} selected={target.key === view.baseline}>{target.label}</option>
			{/each}
		</select>

		<label class="filt-l" for="metric">metric</label>
		<select name="metric" id="metric" class="sel" onchange={view.updateComparison}>
			{#each compareMetricOptions as metric}
				<option value={metric.value} selected={metric.value === view.metric}>{metric.label}</option>
			{/each}
		</select>

		<span class="spacer"></span>
		<button type="submit" class="pill on">compare</button>
	</form>

	{#if view.cohort}
		<div class="filt sub-filt">
			<span class="filt-l">
				{view.cohort.run_ids.length} shards / {view.cohort.targets.length} target ids / {view.cohort.result_count} results / {fmtDuration(view.cohort.start, view.cohort.end)}
			</span>
			<span class="spacer"></span>
			<span class="filt-l">{suiteLabel(view.cohort.suite)} / {view.cohort.class}</span>
		</div>
	{/if}

	{#if view.items}
		<section class="sec">
			<div class="sec-h">
				<span>{view.metric} target ranking</span>
				<span class="mu">{view.items.length} comparable results</span>
			</div>
			{#if view.items.length === 0}
				<div class="empty">
					<p class="empty-text">No comparable target results found for this metric.</p>
				</div>
			{:else}
				<div class="table-scroll">
					<table class="t">
						<thead>
							<tr>
								<th>target</th>
								<th>group</th>
								<th>runner</th>
								<th class="n">value</th>
								<th class="n">baseline</th>
								<th class="n">delta</th>
								<th class="n">pct</th>
								<th class="n">err</th>
								<th style="width: 180px">change</th>
							</tr>
						</thead>
						<tbody>
							{#each view.items as item}
								{@const cls = view.deltaClass(item.delta_pct)}
								<tr>
									<td>
										{item.target_name}
										<div class="mu mono">{item.target_id}</div>
									</td>
									<td class="mu">{item.group ?? 'other'}</td>
									<td class="mu">{item.runner_os}</td>
									<td class="n">{view.formatMetricValue(item.value)}</td>
									<td class="n fade">{view.formatMetricValue(item.baseline_value)}</td>
									<td class="n {cls}">{view.formatMetricValue(item.delta)}</td>
									<td class="n {cls}">{fmtDelta(item.delta_pct)}</td>
									<td class="n mu">{fmtPct(item.err)}</td>
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
	{:else if view.cohort}
		<div class="empty">
			<p class="empty-text">Select a benchmark set and baseline target.</p>
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

	.sub-filt {
		margin-top: -10px;
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
