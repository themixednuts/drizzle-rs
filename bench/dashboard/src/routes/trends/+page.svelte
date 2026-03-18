<script lang="ts">
	import { page } from '$app/state';
	import { fmtRps, fmtLatency, fmtCpu, fmtPct } from '$lib/format';
	import TrendChart from '$lib/components/TrendChart.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const suite = $derived(page.url.searchParams.get('suite'));
	const target = $derived(page.url.searchParams.get('target'));
	const suites = $derived(data.suites);
	const targets = $derived(data.targets);
	const trends = $derived(data.trends);

	function buildUrl(s: string | null, t: string | null): string {
		const params = new URLSearchParams();
		if (s) params.set('suite', s);
		if (t) params.set('target', t);
		const qs = params.toString();
		return '/trends' + (qs ? '?' + qs : '');
	}
</script>

<svelte:head>
	<title>Trends - drizzle-rs bench</title>
</svelte:head>

<div class="container">
	<div class="page-header">
		<h1 class="page-title">Performance Trends</h1>
		<p class="page-desc">Track how a target's performance evolves across benchmark runs</p>
	</div>

	<!-- Selectors -->
	<div class="selector-bar">
		<div class="select-group">
			<span class="select-label">Suite</span>
			<div class="filter-pills">
				<a href={buildUrl(null, target)} class="filter-pill" class:active={!suite}>All</a>
				{#each suites as s}
					<a href={buildUrl(s, target)} class="filter-pill" class:active={suite === s}>{s}</a>
				{/each}
			</div>
		</div>

		<div class="select-group">
			<span class="select-label">Target</span>
			<select
				class="select mono"
				value={target ?? ''}
				onchange={(e) => {
					const val = (e.target as HTMLSelectElement).value;
					window.location.href = buildUrl(suite, val || null);
				}}
			>
				<option value="">Select target...</option>
				{#each targets as t}
					<option value={t}>{t}</option>
				{/each}
			</select>
		</div>
	</div>

	{#if !target}
		<div class="empty">
			<p class="empty-text">Select a target to view performance trends</p>
		</div>
	{:else if trends.length === 0}
			<div class="empty">
				<p class="empty-text">No trend data for {target}</p>
				<p class="empty-sub">Successful runs with this target will appear here</p>
			</div>
		{:else}
			<div class="trend-header">
				<h2 class="trend-target mono">{target}</h2>
				<span class="trend-count">{trends.length} runs</span>
			</div>

			<!-- Summary strip: latest values -->
			{@const latest = trends[trends.length - 1]}
			{@const prev = trends.length > 1 ? trends[trends.length - 2] : null}
			<div class="summary-strip">
				<div class="summary-item">
					<span class="metric-label">Latest RPS</span>
					<span class="metric-value">{fmtRps(latest.rps_avg)}</span>
					{#if prev}
						{@const d = (latest.rps_avg - prev.rps_avg) / (prev.rps_avg || 1)}
						<span class="metric-delta" class:delta-positive={d > 0.005} class:delta-negative={d < -0.005} class:delta-neutral={Math.abs(d) <= 0.005}>
							{d >= 0 ? '+' : ''}{(d * 100).toFixed(1)}%
						</span>
					{/if}
				</div>
				<div class="summary-item">
					<span class="metric-label">Latest P95</span>
					<span class="metric-value">{fmtLatency(latest.latency_p95)}</span>
					{#if prev}
						{@const d = (latest.latency_p95 - prev.latency_p95) / (prev.latency_p95 || 1)}
						<span class="metric-delta" class:delta-positive={d < -0.005} class:delta-negative={d > 0.005} class:delta-neutral={Math.abs(d) <= 0.005}>
							{d >= 0 ? '+' : ''}{(d * 100).toFixed(1)}%
						</span>
					{/if}
				</div>
				<div class="summary-item">
					<span class="metric-label">Latest CPU</span>
					<span class="metric-value">{fmtCpu(latest.cpu_avg)}</span>
				</div>
				<div class="summary-item">
					<span class="metric-label">Error Rate</span>
					<span class="metric-value">{fmtPct(latest.err)}</span>
				</div>
			</div>

			<!-- Charts -->
			<div class="charts">
				<TrendChart
					points={trends}
					metric="rps_avg"
					label="Requests per Second (avg)"
					color="var(--accent)"
					formatValue={fmtRps}
				/>
				<TrendChart
					points={trends}
					metric="latency_p95"
					label="P95 Latency"
					color="var(--cyan)"
					formatValue={fmtLatency}
				/>
				<TrendChart
					points={trends}
					metric="latency_p99"
					label="P99 Latency"
					color="var(--blue)"
					formatValue={fmtLatency}
				/>
				<TrendChart
					points={trends}
					metric="cpu_avg"
					label="CPU Usage (avg)"
					color="var(--green)"
					formatValue={fmtCpu}
				/>
			</div>

			<!-- Run table -->
			<div class="run-table">
				<div class="table-header">
					<span class="th">Commit</span>
					<span class="th th-right">RPS avg</span>
					<span class="th th-right">RPS peak</span>
					<span class="th th-right">P95</span>
					<span class="th th-right">P99</span>
					<span class="th th-right">CPU</span>
					<span class="th th-right">Err</span>
				</div>
				{#each [...trends].reverse() as point, i}
					<a href="/runs/{point.run_id}" class="table-row" style="--delay: {i * 25}ms">
						<span class="td mono git-col">{point.git.slice(0, 7)}</span>
						<span class="td td-right mono">{fmtRps(point.rps_avg)}</span>
						<span class="td td-right mono">{fmtRps(point.rps_peak)}</span>
						<span class="td td-right mono">{fmtLatency(point.latency_p95)}</span>
						<span class="td td-right mono">{fmtLatency(point.latency_p99)}</span>
						<span class="td td-right mono">{fmtCpu(point.cpu_avg)}</span>
						<span class="td td-right mono">{fmtPct(point.err)}</span>
					</a>
				{/each}
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

	.selector-bar {
		display: flex;
		gap: 24px;
		align-items: flex-end;
		margin-bottom: 32px;
		padding-bottom: 20px;
		border-bottom: 1px solid var(--border);
		flex-wrap: wrap;
	}

	.select-group {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.select-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
	}

	.filter-pills {
		display: flex;
		gap: 4px;
	}

	.filter-pill {
		padding: 4px 12px;
		border-radius: 100px;
		font-size: 12px;
		font-weight: 500;
		color: var(--text-secondary);
		border: 1px solid var(--border);
		transition: all 0.15s;
	}

	.filter-pill:hover {
		border-color: var(--border-accent);
		color: var(--text-primary);
	}

	.filter-pill.active {
		background: var(--accent-dim);
		border-color: var(--accent);
		color: var(--accent);
	}

	.select {
		padding: 6px 12px;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-primary);
		font-size: 12px;
		appearance: none;
		cursor: pointer;
		min-width: 220px;
	}

	.select:focus {
		outline: none;
		border-color: var(--accent);
	}

	.trend-header {
		display: flex;
		align-items: baseline;
		gap: 12px;
		margin-bottom: 20px;
	}

	.trend-target {
		font-size: 20px;
		font-weight: 600;
		color: var(--cyan);
	}

	.trend-count {
		font-size: 12px;
		color: var(--text-muted);
		font-family: var(--font-mono);
		padding: 2px 8px;
		background: var(--bg-raised);
		border-radius: var(--radius);
	}

	.summary-strip {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 16px;
		margin-bottom: 32px;
		padding: 16px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
	}

	.summary-item {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.metric-delta {
		font-family: var(--font-mono);
		font-size: 12px;
	}

	.charts {
		margin-bottom: 32px;
	}

	.run-table {
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		overflow: hidden;
		margin-bottom: 32px;
	}

	.table-header {
		display: grid;
		grid-template-columns: 1fr repeat(6, 90px);
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

	.table-row {
		display: grid;
		grid-template-columns: 1fr repeat(6, 90px);
		padding: 10px 18px;
		border-bottom: 1px solid var(--border);
		animation: fadeSlideIn 0.3s ease both;
		animation-delay: var(--delay);
		transition: background 0.15s;
	}

	.table-row:last-child { border-bottom: none; }
	.table-row:hover { background: var(--bg-hover); }

	.td { font-size: 12px; display: flex; align-items: center; }
	.td-right { justify-content: flex-end; }

	.git-col {
		color: var(--accent);
		font-weight: 500;
	}

	.empty {
		text-align: center;
		padding: 80px 24px;
	}

	.empty-text {
		font-size: 18px;
		font-weight: 600;
		color: var(--text-secondary);
	}

	.empty-sub {
		font-size: 13px;
		color: var(--text-muted);
		margin-top: 8px;
	}

	@keyframes fadeSlideIn {
		from { opacity: 0; transform: translateY(6px); }
		to { opacity: 1; transform: translateY(0); }
	}

	@media (max-width: 768px) {
		.page-title {
			font-size: 22px;
		}

		.selector-bar {
			flex-direction: column;
			gap: 12px;
			align-items: flex-start;
		}

		.select {
			min-width: 0;
			width: 100%;
		}

		.summary-strip {
			grid-template-columns: repeat(2, 1fr);
		}

		.table-header,
		.table-row {
			grid-template-columns: 1fr repeat(3, 64px);
		}

		/* Hide less critical columns on mobile */
		.table-header .th:nth-child(3),
		.table-header .th:nth-child(5),
		.table-header .th:nth-child(7),
		.table-row .td:nth-child(3),
		.table-row .td:nth-child(5),
		.table-row .td:nth-child(7) {
			display: none;
		}
	}
</style>
