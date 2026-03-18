<script lang="ts">
	import { fmtRps, fmtLatency, fmtPct, fmtCpu, fmtDate, fmtDuration, shortHash } from '$lib/format';
	import { loadTimeseries } from '$lib/api.remote';
	import SparkLine from '$lib/components/SparkLine.svelte';
	import LatencyBars from '$lib/components/LatencyBars.svelte';
	import type { Summary } from '$lib/types';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	let selectedMetric = $state<'rps' | 'latency' | 'cpu' | 'mem'>('rps');
	const m = $derived(data.manifest);
	const summaries = $derived(data.summaries);

	const groupColors: Record<string, string> = {
		'drizzle-rs': 'var(--accent)',
		'drizzle-orm': 'var(--cyan)',
		'prisma': 'var(--green)',
		'bun-sql': 'var(--text-secondary)',
		'spacetimedb': 'var(--purple)',
	};

	function groupSummaries(summaries: Summary[]): [string, Summary[]][] {
		const map = new Map<string, Summary[]>();
		for (const s of summaries) {
			const g = s.group ?? 'other';
			if (!map.has(g)) map.set(g, []);
			map.get(g)!.push(s);
		}
		return [...map.entries()];
	}
</script>

<svelte:head>
	<title>Run Detail - drizzle-rs bench</title>
</svelte:head>

<div class="container">
	<!-- Run header -->
	<div class="run-header">
		<div class="run-header-top">
			<a href="/" class="back-link">
				<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
					<path d="M10 12L6 8l4-4" />
				</svg>
				Runs
			</a>
			<span class="badge badge--{m.status}">{m.status}</span>
		</div>

		<h1 class="run-title mono">{m.run_id}</h1>

		<div class="run-info-grid">
			<div class="info-item">
				<span class="info-label">Suite</span>
				<span class="suite-tag">{m.suite}</span>
			</div>
			<div class="info-item">
				<span class="info-label">Commit</span>
				<span class="info-value mono">{shortHash(m.git)}</span>
			</div>
			<div class="info-item">
				<span class="info-label">Started</span>
				<span class="info-value">{fmtDate(m.start)}</span>
			</div>
			<div class="info-item">
				<span class="info-label">Duration</span>
				<span class="info-value mono">{fmtDuration(m.start, m.end)}</span>
			</div>
			<div class="info-item">
				<span class="info-label">Trials</span>
				<span class="info-value mono">{m.trials.count} ({m.trials.aggregate})</span>
			</div>
			<div class="info-item">
				<span class="info-label">Runner</span>
				<span class="info-value mono">{m.runner.class} / {m.runner.cores}c / {m.runner.mem_gb}GB</span>
			</div>
			{#if m.seed != null}
			<div class="info-item">
				<span class="info-label">Seed</span>
				<span class="info-value mono">{m.seed}</span>
			</div>
			{/if}
			<div class="info-item">
				<span class="info-label">Headroom</span>
				<span class="info-value mono">CPU {fmtCpu(m.runner.headroom.cpu_peak)} / Net {fmtPct(m.runner.headroom.net_peak)}</span>
			</div>
		</div>

		<!-- Load configuration -->
		{#if m.load}
		<div class="detail-panel">
			<h3 class="detail-title">Load Profile</h3>
			<div class="detail-grid">
				<div class="detail-item">
					<span class="info-label">Executor</span>
					<span class="info-value mono">{m.load.executor}</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Stages</span>
					<span class="info-value mono">{m.load.stages}</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Duration</span>
					<span class="info-value mono">{Math.floor(m.load.duration_s / 60)}m {m.load.duration_s % 60}s</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Max VUs</span>
					<span class="info-value mono">{m.load.max_vus.toLocaleString()}</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Requests</span>
					<span class="info-value mono">{m.load.requests.toLocaleString()}</span>
				</div>
			</div>
		</div>
		{/if}

		<!-- Dataset configuration -->
		{#if m.dataset}
		<div class="detail-panel">
			<h3 class="detail-title">Dataset</h3>
			<div class="detail-grid">
				<div class="detail-item">
					<span class="info-label">Customers</span>
					<span class="info-value mono">{m.dataset.customers.toLocaleString()}</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Employees</span>
					<span class="info-value mono">{m.dataset.employees.toLocaleString()}</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Orders</span>
					<span class="info-value mono">{m.dataset.orders.toLocaleString()}</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Suppliers</span>
					<span class="info-value mono">{m.dataset.suppliers.toLocaleString()}</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Products</span>
					<span class="info-value mono">{m.dataset.products.toLocaleString()}</span>
				</div>
				<div class="detail-item">
					<span class="info-label">Details/Order</span>
					<span class="info-value mono">~{m.dataset.details_per_order}</span>
				</div>
			</div>
		</div>
		{/if}
	</div>

	<!-- Target summaries -->
	<section class="section">
		<h2 class="section-title">Targets</h2>

		{#each groupSummaries(summaries) as [groupName, groupItems]}
			<div class="group-banner" style="--group-color: {groupColors[groupName] ?? 'var(--text-muted)'}">
				<span class="group-dot"></span>
				<span class="group-name">{groupName}</span>
				<span class="group-count">{groupItems.length}</span>
			</div>
			<div class="target-grid">
			{#each groupItems as summary, idx}
				{@const p = summary.primary}
				<div class="target-card card" style="--delay: {idx * 60}ms">
					<div class="target-header">
						<h3 class="target-name mono">{summary.target_id}</h3>
						{#if p.err > 0}
							<span class="badge badge--failed">{fmtPct(p.err)} err</span>
						{:else}
							<span class="badge badge--success">0% err</span>
						{/if}
					</div>

					<div class="metrics-row">
						<div class="metric">
							<span class="metric-label">RPS avg</span>
							<span class="metric-value">{fmtRps(p.rps.avg)}</span>
							<span class="metric-sub">peak {fmtRps(p.rps.peak)}</span>
						</div>
						<div class="metric">
							<span class="metric-label">P95 latency</span>
							<span class="metric-value">{fmtLatency(p.latency.p95)}</span>
							<span class="metric-sub">avg {fmtLatency(p.latency.avg)}</span>
						</div>
						<div class="metric">
							<span class="metric-label">CPU avg</span>
							<span class="metric-value">{fmtCpu(p.cpu.avg)}</span>
							<span class="metric-sub">peak {fmtCpu(p.cpu.peak)}</span>
						</div>
						{#if p.mem}
						<div class="metric">
							<span class="metric-label">Memory avg</span>
							<span class="metric-value">{p.mem.avg.toFixed(1)} MB</span>
							<span class="metric-sub">peak {p.mem.peak.toFixed(1)} MB</span>
						</div>
						{/if}
					</div>

					<!-- Latency distribution -->
					<div class="latency-section">
						<span class="metric-label">Latency distribution</span>
						<LatencyBars latency={p.latency} />
					</div>

					<!-- Sparkline (lazy-loaded via remote function) -->
					<div class="sparkline-section">
						<div class="sparkline-tabs">
							<button
								class="spark-tab"
								class:active={selectedMetric === 'rps'}
								onclick={() => selectedMetric = 'rps'}
							>RPS</button>
							<button
								class="spark-tab"
								class:active={selectedMetric === 'latency'}
								onclick={() => selectedMetric = 'latency'}
							>P95</button>
							<button
								class="spark-tab"
								class:active={selectedMetric === 'cpu'}
								onclick={() => selectedMetric = 'cpu'}
							>CPU</button>
							{#if p.mem}
							<button
								class="spark-tab"
								class:active={selectedMetric === 'mem'}
								onclick={() => selectedMetric = 'mem'}
							>Mem</button>
							{/if}
						</div>
						<svelte:boundary>
							{#snippet pending()}
								<div class="skeleton" style="height: 48px; margin-top: 4px;"></div>
							{/snippet}

							{@const ts = await loadTimeseries({ runId: m.run_id, targetId: summary.target_id })}
							{#if ts}
								<SparkLine points={ts.points} metric={selectedMetric} />
							{:else}
								<div class="spark-empty">No timeseries data</div>
							{/if}
						</svelte:boundary>
					</div>

					<!-- Spread -->
					{#if summary.spread.trials > 1}
						<div class="spread-section">
							<span class="metric-label">Spread ({summary.spread.trials} trials, {summary.spread.aggregate})</span>
							<div class="spread-grid">
								<div class="spread-item">
									<span class="spread-label">RPS</span>
									<span class="spread-range mono">{fmtRps(summary.spread.rps.min)} - {fmtRps(summary.spread.rps.max)}</span>
								</div>
								<div class="spread-item">
									<span class="spread-label">P95</span>
									<span class="spread-range mono">{fmtLatency(summary.spread.p95.min)} - {fmtLatency(summary.spread.p95.max)}</span>
								</div>
								{#if summary.spread.ci95?.rps}
									<div class="spread-item">
										<span class="spread-label">CI95 RPS</span>
										<span class="spread-range mono">{fmtRps(summary.spread.ci95.rps.min)} - {fmtRps(summary.spread.ci95.rps.max)}</span>
									</div>
								{/if}
							</div>
						</div>
					{/if}

					<!-- Saturation -->
					<div class="sat-section">
						<span class="metric-label">Saturation point</span>
						<div class="sat-row">
							<span class="sat-item mono">knee RPS: {fmtRps(summary.saturation.knee_rps)}</span>
							<span class="sat-item mono">knee P95: {fmtLatency(summary.saturation.knee_p95)}</span>
						</div>
					</div>
				</div>
			{/each}
			</div>
		{/each}
	</section>
</div>

<style>
	.run-header {
		margin-bottom: 40px;
	}

	.run-header-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 12px;
	}

	.back-link {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 13px;
		color: var(--text-secondary);
		transition: color 0.15s;
	}

	.back-link:hover {
		color: var(--text-primary);
	}

	.run-title {
		font-size: 20px;
		font-weight: 600;
		margin-bottom: 16px;
		color: var(--accent);
	}

	.run-info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 12px;
		padding: 16px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
	}

	.info-item {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.info-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
	}

	.info-value {
		font-size: 13px;
		color: var(--text-primary);
	}

	.detail-panel {
		margin-top: 12px;
		padding: 14px 16px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
	}

	.detail-title {
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
		margin-bottom: 10px;
	}

	.detail-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
		gap: 10px;
	}

	.detail-item {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.section {
		margin-bottom: 40px;
	}

	.section-title {
		font-size: 18px;
		font-weight: 600;
		margin-bottom: 16px;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.section-title::before {
		content: '';
		display: block;
		width: 3px;
		height: 18px;
		background: var(--accent);
		border-radius: 2px;
	}

	.group-banner {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 14px;
		margin-top: 24px;
		padding: 10px 16px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-left: 3px solid var(--group-color);
		border-radius: var(--radius);
	}

	.group-banner:first-child {
		margin-top: 0;
	}

	.group-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--group-color);
	}

	.group-name {
		font-size: 14px;
		font-weight: 600;
		color: var(--group-color);
		text-transform: capitalize;
	}

	.group-count {
		font-size: 11px;
		font-family: var(--font-mono);
		color: var(--text-muted);
		padding: 1px 6px;
		background: var(--bg-raised);
		border-radius: var(--radius);
	}

	.target-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
		gap: 16px;
	}

	.target-card {
		padding: 20px;
		animation: fadeSlideIn 0.4s ease both;
		animation-delay: var(--delay);
	}

	.target-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 20px;
		padding-bottom: 12px;
		border-bottom: 1px solid var(--border);
	}

	.target-name {
		font-size: 16px;
		font-weight: 600;
		color: var(--cyan);
	}

	.metrics-row {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 16px;
		margin-bottom: 20px;
	}

	.metric {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.latency-section,
	.sparkline-section,
	.spread-section,
	.sat-section {
		margin-top: 16px;
		padding-top: 14px;
		border-top: 1px solid var(--border);
	}

	.sparkline-tabs {
		display: flex;
		gap: 2px;
		margin: 8px 0;
	}

	.spark-tab {
		padding: 3px 10px;
		border: none;
		border-radius: var(--radius);
		background: transparent;
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 11px;
		cursor: pointer;
		transition: all 0.15s;
	}

	.spark-tab:hover {
		color: var(--text-secondary);
		background: var(--bg-hover);
	}

	.spark-tab.active {
		color: var(--accent);
		background: rgba(212, 160, 23, 0.1);
	}

	.spark-empty {
		height: 48px;
		display: flex;
		align-items: center;
		font-size: 11px;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.spread-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 8px;
		margin-top: 8px;
	}

	.spread-item {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.spread-label {
		font-size: 11px;
		color: var(--text-muted);
		font-weight: 500;
	}

	.spread-range {
		font-size: 12px;
		color: var(--text-secondary);
	}

	.sat-row {
		display: flex;
		gap: 24px;
		margin-top: 6px;
	}

	.sat-item {
		font-size: 12px;
		color: var(--text-secondary);
	}

	@keyframes fadeSlideIn {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@media (max-width: 768px) {
		.run-title {
			font-size: 16px;
			word-break: break-all;
		}

		.run-info-grid {
			grid-template-columns: repeat(2, 1fr);
		}

		.target-grid {
			grid-template-columns: 1fr;
		}

		.metrics-row {
			grid-template-columns: 1fr;
			gap: 12px;
		}

		.sat-row {
			flex-direction: column;
			gap: 8px;
		}
	}
</style>
