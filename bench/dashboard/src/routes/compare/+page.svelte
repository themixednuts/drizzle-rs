<script lang="ts">
	import { page } from '$app/state';
	import { compareRuns } from '$lib/compare-form.remote';
	import {
		compareMetricOptions,
		isHigherBetterMetric,
		parseCompareMetric
	} from '$lib/compare';
	import { fmtDelta } from '$lib/format';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	class ComparePageState {
		#data: () => PageData;
		base = $derived(page.url.searchParams.get('base'));
		head = $derived(page.url.searchParams.get('head'));
		metric = $derived(parseCompareMetric(page.url.searchParams.get('metric')));

		constructor(data: () => PageData) {
			this.#data = data;
		}

		get runs() {
			return this.#data().runs;
		}

		get items() {
			return this.#data().items;
		}

		deltaClass = (pct: number): string => {
			if (Math.abs(pct) < 0.005) return 'delta-neutral';
			const positive = pct > 0;
			const good = isHigherBetterMetric(this.metric) ? positive : !positive;
			return good ? 'delta-positive' : 'delta-negative';
		};

		formatMetricValue = (value: number): string => {
			if (this.metric.startsWith('rps')) {
				if (value >= 1_000_000) return (value / 1_000_000).toFixed(1) + 'M';
				if (value >= 1_000) return (value / 1_000).toFixed(1) + 'k';
				return value.toFixed(0);
			}
			if (this.metric.startsWith('latency')) {
				if (value >= 1_000) return (value / 1_000).toFixed(2) + 's';
				if (value >= 1) return value.toFixed(1) + 'ms';
				return (value * 1_000).toFixed(0) + 'us';
			}
			if (this.metric.startsWith('cpu')) return value.toFixed(1) + '%';
			if (this.metric === 'err') return (value * 100).toFixed(2) + '%';
			return value.toFixed(2);
		};

		barStyle = (deltaPct: number): string => {
			const side = deltaPct >= 0 ? 'left: 50%' : 'right: 50%';
			return `width: ${Math.min(Math.abs(deltaPct) * 100, 100)}%; ${side}`;
		};
	}

	const view = new ComparePageState(() => data);
</script>

<svelte:head>
	<title>Compare - drizzle-rs bench</title>
</svelte:head>

<div class="container">
	<div class="page-header">
		<h1 class="page-title">Compare Runs</h1>
		<p class="page-desc">Side-by-side metric comparison between two benchmark runs</p>
	</div>

	<!-- Selectors -->
	<form class="compare-form" {...compareRuns}>
		<div class="select-group">
			<label class="select-label" for="base">Base run</label>
			<select name="base" id="base" class="select mono" value={view.base ?? ''}>
				<option value="">Select base...</option>
				{#each view.runs as run}
					<option value={run.run_id} selected={run.run_id === view.base}>
						{run.run_id} ({run.suite})
					</option>
				{/each}
			</select>
		</div>

		<div class="select-arrow">
			<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5">
				<path d="M8 6l4 4-4 4" />
			</svg>
		</div>

		<div class="select-group">
			<label class="select-label" for="head">Head run</label>
			<select name="head" id="head" class="select mono" value={view.head ?? ''}>
				<option value="">Select head...</option>
				{#each view.runs as run}
					<option value={run.run_id} selected={run.run_id === view.head}>
						{run.run_id} ({run.suite})
					</option>
				{/each}
			</select>
		</div>

		<div class="select-group">
			<label class="select-label" for="metric">Metric</label>
			<select name="metric" id="metric" class="select">
				{#each compareMetricOptions as m}
					<option value={m.value} selected={m.value === view.metric}>{m.label}</option>
				{/each}
			</select>
		</div>

		<button type="submit" class="compare-btn">Compare</button>
	</form>

	<!-- Results -->
	{#if view.items}
			{#if view.items.length === 0}
				<div class="empty">
					<p class="empty-text">No comparable targets found</p>
				</div>
			{:else}
				<div class="results-table">
					<div class="table-header">
						<span class="th">Target</span>
						<span class="th th-right">Base</span>
						<span class="th th-right">Head</span>
						<span class="th th-right">Delta</span>
						<span class="th th-center">Change</span>
					</div>
					{#each view.items as item, i}
						{@const cls = view.deltaClass(item.delta_pct)}
						<div class="table-row" style="--delay: {i * 40}ms">
							<span class="td target-col mono">{item.target_id}</span>
							<span class="td td-right mono">{view.formatMetricValue(item.base_value)}</span>
							<span class="td td-right mono">{view.formatMetricValue(item.head_value)}</span>
							<span class="td td-right mono {cls}">{fmtDelta(item.delta_pct)}</span>
							<span class="td td-center">
								<div class="bar-wrap">
									<div
										class="bar {cls}"
										style={view.barStyle(item.delta_pct)}
									></div>
									<div class="bar-center"></div>
								</div>
							</span>
						</div>
					{/each}
				</div>
			{/if}
	{:else if view.base || view.head}
		<div class="empty">
			<p class="empty-text">Select both a base and head run to compare</p>
		</div>
	{/if}
</div>

<style>
	.page-header {
		margin-bottom: 32px;
	}

	.page-title {
		font-size: 28px;
		font-weight: 700;
		letter-spacing: -0.02em;
	}

	.page-desc {
		color: var(--text-secondary);
		font-size: 14px;
		margin-top: 4px;
	}

	.compare-form {
		display: flex;
		align-items: flex-end;
		gap: 16px;
		margin-bottom: 32px;
		padding: 20px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		flex-wrap: wrap;
	}

	.select-group {
		display: flex;
		flex-direction: column;
		gap: 6px;
		flex: 1;
		min-width: 200px;
	}

	.select-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
	}

	.select {
		padding: 8px 12px;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-primary);
		font-size: 12px;
		appearance: none;
		cursor: pointer;
	}

	.select:focus {
		outline: none;
		border-color: var(--accent);
	}

	.select-arrow {
		color: var(--text-muted);
		padding-bottom: 6px;
	}

	.compare-btn {
		padding: 8px 20px;
		background: var(--accent-dim);
		border: 1px solid var(--accent);
		border-radius: var(--radius);
		color: var(--accent);
		font-weight: 600;
		font-size: 13px;
		cursor: pointer;
		transition: all 0.15s;
	}

	.compare-btn:hover {
		background: var(--accent);
		color: var(--bg-root);
	}

	.results-table {
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}

	.table-header {
		display: grid;
		grid-template-columns: 1fr 120px 120px 100px 200px;
		padding: 10px 18px;
		background: var(--bg-raised);
		border-bottom: 1px solid var(--border);
	}

	.th {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
	}

	.th-right { text-align: right; }
	.th-center { text-align: center; }

	.table-row {
		display: grid;
		grid-template-columns: 1fr 120px 120px 100px 200px;
		padding: 12px 18px;
		border-bottom: 1px solid var(--border);
		animation: fadeSlideIn 0.3s ease both;
		animation-delay: var(--delay);
		transition: background 0.15s;
	}

	.table-row:last-child { border-bottom: none; }
	.table-row:hover { background: var(--bg-hover); }

	.td { font-size: 13px; display: flex; align-items: center; }
	.td-right { justify-content: flex-end; }
	.td-center { justify-content: center; }

	.target-col {
		font-weight: 500;
		color: var(--cyan);
	}

	.bar-wrap {
		width: 100%;
		height: 16px;
		position: relative;
		border-radius: 3px;
		overflow: hidden;
		background: var(--bg-root);
	}

	.bar {
		position: absolute;
		top: 2px;
		bottom: 2px;
		border-radius: 2px;
		transition: width 0.4s ease;
	}

	.bar.delta-positive { background: var(--green); opacity: 0.6; }
	.bar.delta-negative { background: var(--red); opacity: 0.6; }
	.bar.delta-neutral { background: var(--text-muted); opacity: 0.3; }

	.bar-center {
		position: absolute;
		left: 50%;
		top: 0;
		bottom: 0;
		width: 1px;
		background: var(--border-accent);
	}

	.empty {
		text-align: center;
		padding: 60px 24px;
	}

	.empty-text {
		font-size: 16px;
		color: var(--text-secondary);
	}

	@keyframes fadeSlideIn {
		from { opacity: 0; transform: translateY(6px); }
		to { opacity: 1; transform: translateY(0); }
	}

	@media (max-width: 768px) {
		.page-title {
			font-size: 22px;
		}

		.compare-form {
			flex-direction: column;
		}

		.select-arrow {
			transform: rotate(90deg);
			align-self: center;
		}

		.select-group {
			min-width: 0;
			width: 100%;
		}

		.table-header,
		.table-row {
			grid-template-columns: 1fr 80px 80px 70px;
		}

		/* Hide bar chart column on mobile */
		.table-header .th:nth-child(5),
		.table-row .td:nth-child(5) {
			display: none;
		}
	}
</style>
