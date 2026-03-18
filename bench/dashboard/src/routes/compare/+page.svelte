<script lang="ts">
	import { page } from '$app/state';
	import { fmtDelta } from '$lib/format';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const metrics = [
		{ value: 'rps.avg', label: 'RPS (avg)' },
		{ value: 'rps.peak', label: 'RPS (peak)' },
		{ value: 'latency.avg', label: 'Latency (avg)' },
		{ value: 'latency.p95', label: 'Latency (p95)' },
		{ value: 'latency.p99', label: 'Latency (p99)' },
		{ value: 'cpu.avg', label: 'CPU (avg)' },
		{ value: 'cpu.peak', label: 'CPU (peak)' },
		{ value: 'err', label: 'Error rate' }
	];

	// For RPS, higher is better; for latency/cpu/err, lower is better
	function isHigherBetter(metric: string): boolean {
		return metric.startsWith('rps');
	}

	function deltaClass(pct: number, metric: string): string {
		if (Math.abs(pct) < 0.005) return 'delta-neutral';
		const positive = pct > 0;
		const good = isHigherBetter(metric) ? positive : !positive;
		return good ? 'delta-positive' : 'delta-negative';
	}

	function fmtMetricValue(val: number, metric: string): string {
		if (metric.startsWith('rps')) {
			if (val >= 1_000_000) return (val / 1_000_000).toFixed(1) + 'M';
			if (val >= 1_000) return (val / 1_000).toFixed(1) + 'k';
			return val.toFixed(0);
		}
		if (metric.startsWith('latency')) {
			if (val >= 1_000) return (val / 1_000).toFixed(2) + 's';
			if (val >= 1) return val.toFixed(1) + 'ms';
			return (val * 1_000).toFixed(0) + 'us';
		}
		if (metric.startsWith('cpu')) return val.toFixed(1) + '%';
		if (metric === 'err') return (val * 100).toFixed(2) + '%';
		return val.toFixed(2);
	}

	const base = $derived(page.url.searchParams.get('base'));
	const head = $derived(page.url.searchParams.get('head'));
	const metric = $derived(page.url.searchParams.get('metric') ?? 'rps.avg');
	const runs = $derived(data.runs);
	const items = $derived(data.items);
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
	<form class="compare-form" method="get">
		<div class="select-group">
			<label class="select-label" for="base">Base run</label>
			<select name="base" id="base" class="select mono" value={base ?? ''}>
				<option value="">Select base...</option>
				{#each runs as run}
					<option value={run.run_id} selected={run.run_id === base}>
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
			<select name="head" id="head" class="select mono" value={head ?? ''}>
				<option value="">Select head...</option>
				{#each runs as run}
					<option value={run.run_id} selected={run.run_id === head}>
						{run.run_id} ({run.suite})
					</option>
				{/each}
			</select>
		</div>

		<div class="select-group">
			<label class="select-label" for="metric">Metric</label>
			<select name="metric" id="metric" class="select">
				{#each metrics as m}
					<option value={m.value} selected={m.value === metric}>{m.label}</option>
				{/each}
			</select>
		</div>

		<button type="submit" class="compare-btn">Compare</button>
	</form>

	<!-- Results -->
	{#if items}
			{#if items.length === 0}
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
					{#each items as item, i}
						{@const cls = deltaClass(item.delta_pct, metric)}
						<div class="table-row" style="--delay: {i * 40}ms">
							<span class="td target-col mono">{item.target_id}</span>
							<span class="td td-right mono">{fmtMetricValue(item.base_value, metric)}</span>
							<span class="td td-right mono">{fmtMetricValue(item.head_value, metric)}</span>
							<span class="td td-right mono {cls}">{fmtDelta(item.delta_pct)}</span>
							<span class="td td-center">
								<div class="bar-wrap">
									<div
										class="bar {cls}"
										style="width: {Math.min(Math.abs(item.delta_pct) * 100, 100)}%; {item.delta_pct >= 0 ? 'left: 50%' : 'right: 50%'}"
									></div>
									<div class="bar-center"></div>
								</div>
							</span>
						</div>
					{/each}
				</div>
			{/if}
	{:else if base || head}
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
