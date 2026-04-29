<script lang="ts">
	import { compareCategoryOptions } from '$lib/compare';
	import { compareTargets } from '$lib/compare-form.remote';
	import { fmtDate, fmtDuration, runDisplayName, shortHash, suiteLabel } from '$lib/format';
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
			<div class="ph-sub">rank every target result in one benchmark set by category shape and sample variance</div>
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

		<label class="filt-l" for="metric">category</label>
		<select name="metric" id="metric" class="sel" onchange={view.updateComparison}>
			{#each compareCategoryOptions as metric}
				<option value={metric.value} selected={metric.value === view.category}>{metric.label}</option>
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
				<span>{view.categoryLabel} target ranking</span>
				<span class="mu">{view.items.length} comparable results / bar shows sample variance across trials</span>
			</div>
			{#if view.items.length === 0}
				<div class="empty">
					<p class="empty-text">No comparable target results found for this category.</p>
				</div>
			{:else}
				<div class="table-scroll">
					<table class="t">
						<thead>
							<tr>
								<th>target</th>
								{#each view.columns as column}
									<th class="n">{column.label}</th>
								{/each}
								{#if view.showErrorColumn}
									<th class="n">err</th>
								{/if}
								<th style="width: 320px">variance</th>
							</tr>
						</thead>
						<tbody>
							{#each view.items as item}
								{@const display = view.targetDisplay(item)}
								<tr
									class={view.rowClass(item)}
									onpointerenter={() => view.hoverTarget(item)}
									onpointerleave={view.clearHover}
								>
									<td>
										<span class="target-link">{display.name}</span>
										<span class="target-badges" title={item.target_id}>
											{#each display.badges as badge, index}
												{#if index > 0}<span class="target-slash">/</span>{/if}
												<span>{badge}</span>
											{/each}
										</span>
									</td>
									{#each view.columns as column}
										{@const value = view.valueFor(item, column.key)}
										<td class="n">{value ? view.formatValue(value.value) : '-'}</td>
									{/each}
									{#if view.showErrorColumn}
										<td class="n mu">{view.formatValue(item.err, 'err')}</td>
									{/if}
									<td>
										<div class="variance-wrap">
											<div class="variance-track" style={view.varianceStyle(item)}>
												<span class="variance-bar"></span>
											</div>
											<span class="variance-label">{view.varianceLabel(item)}</span>
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
			<p class="empty-text">Select a benchmark set with comparable target results.</p>
		</div>
	{/if}
</main>

<style>
	.compare-form {
		display: grid;
		grid-template-columns:
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

	.variance-wrap {
		display: grid;
		min-width: 260px;
		gap: 5px;
	}

	.variance-track {
		position: relative;
		width: 100%;
		height: 14px;
		background: var(--bg-2);
		overflow: hidden;
	}

	.variance-bar {
		position: absolute;
		top: 3px;
		bottom: 3px;
		left: 0;
		width: var(--variance-width);
		background: linear-gradient(90deg, var(--acc), var(--ink));
	}

	.variance-label {
		color: var(--ink-3);
		font-family: var(--font-mono);
		font-size: 10.5px;
		line-height: 1.25;
	}

	@media (max-width: 760px) {
		.compare-form {
			display: flex;
			align-items: stretch;
			flex-direction: column;
		}
	}
</style>
