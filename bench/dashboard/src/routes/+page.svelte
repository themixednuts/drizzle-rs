<script lang="ts">
	import { page } from '$app/state';
	import { fmtDate, fmtDuration, shortHash } from '$lib/format';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const suite = $derived(page.url.searchParams.get('suite'));
	const status = $derived(page.url.searchParams.get('status'));
	const runs = $derived(data.runs);
	const suites = $derived(data.suites);
	const statuses = $derived(data.statuses);
</script>

<svelte:head>
	<title>drizzle-rs bench</title>
</svelte:head>

<div class="container">
	<div class="page-header">
		<h1 class="page-title">Benchmark Runs</h1>
		<p class="page-desc">Performance tracking for drizzle-rs query builders</p>
	</div>

	<div class="filters">
		<div class="filter-group">
			<span class="filter-label">Suite</span>
			<div class="filter-pills">
				<a
					href="/"
					class="filter-pill"
					class:active={!suite}
				>All</a>
				{#each suites as s}
					<a
						href="/?suite={s}{status ? '&status=' + status : ''}"
						class="filter-pill"
						class:active={suite === s}
					>{s}</a>
				{/each}
			</div>
		</div>
		<div class="filter-group">
			<span class="filter-label">Status</span>
			<div class="filter-pills">
				<a
					href="/{suite ? '?suite=' + suite : ''}"
					class="filter-pill"
					class:active={!status}
				>All</a>
				{#each statuses as st}
					<a
						href="/?{suite ? 'suite=' + suite + '&' : ''}status={st}"
						class="filter-pill"
						class:active={status === st}
					>{st}</a>
				{/each}
			</div>
		</div>
	</div>

	{#if runs.length === 0}
		<div class="empty">
			<svg class="empty-icon" width="48" height="48" viewBox="0 0 48 48" fill="none" stroke="var(--text-muted)" stroke-width="1.5">
				<rect x="8" y="6" width="32" height="36" rx="4" />
				<path d="M16 16h16M16 22h12M16 28h8" stroke-linecap="round" opacity="0.5" />
			</svg>
			<p class="empty-text">No runs found</p>
			<p class="empty-sub">
				{#if suite || status}
					Try adjusting your filters
				{:else}
					Benchmark runs will appear here after CI publishes results to R2
				{/if}
			</p>
		</div>
	{:else}
		<div class="run-list">
			{#each runs as run, i}
				<a href="/runs/{run.run_id}" class="run-row" style="--delay: {i * 30}ms">
					<div class="run-left">
						<span class="dot dot--{run.status}"></span>
						<div class="run-meta">
							<div class="run-id mono">{run.run_id}</div>
							<div class="run-details">
								<span class="suite-tag">{run.suite}</span>
								<span class="badge badge--{run.status}">{run.status}</span>
								<span class="run-git mono">
									<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" opacity="0.5">
										<path d="M8 0a8 8 0 1 0 0 16A8 8 0 0 0 8 0zm3.8 5.3L7.5 11.1a.7.7 0 0 1-1 .1L4.2 9a.7.7 0 0 1 1-1l1.8 1.7 3.8-5.3a.7.7 0 0 1 1 1z"/>
									</svg>
									{shortHash(run.git)}
								</span>
							</div>
						</div>
					</div>
					<div class="run-right">
						<div class="run-targets mono">
							{run.targets.length} target{run.targets.length !== 1 ? 's' : ''}
						</div>
						<div class="run-time">
							<span class="run-date">{fmtDate(run.start)}</span>
							<span class="run-duration mono">{fmtDuration(run.start, run.end)}</span>
						</div>
					</div>
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

	.filters {
		display: flex;
		gap: 24px;
		margin-bottom: 24px;
		padding-bottom: 20px;
		border-bottom: 1px solid var(--border);
	}

	.filter-group {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.filter-label {
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

	.run-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.run-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 18px;
		border-radius: var(--radius);
		border: 1px solid transparent;
		transition: all 0.15s;
		animation: fadeSlideIn 0.3s ease both;
		animation-delay: var(--delay);
	}

	.run-row:hover {
		background: var(--bg-surface);
		border-color: var(--border);
	}

	.run-left {
		display: flex;
		align-items: center;
		gap: 14px;
	}

	.run-meta {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.run-id {
		font-size: 13px;
		font-weight: 500;
		color: var(--text-primary);
	}

	.run-details {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.run-git {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 11px;
		color: var(--text-secondary);
	}

	.run-right {
		display: flex;
		align-items: center;
		gap: 24px;
		text-align: right;
	}

	.run-targets {
		font-size: 12px;
		color: var(--text-secondary);
		padding: 2px 10px;
		background: var(--bg-raised);
		border-radius: var(--radius);
	}

	.run-time {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.run-date {
		font-size: 12px;
		color: var(--text-secondary);
	}

	.run-duration {
		font-size: 11px;
		color: var(--text-muted);
	}

	.empty {
		text-align: center;
		padding: 80px 24px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
	}

	.empty-icon {
		opacity: 0.4;
		margin-bottom: 4px;
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
		.page-title {
			font-size: 22px;
		}

		.filters {
			flex-direction: column;
			gap: 12px;
		}

		.run-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 10px;
			padding: 12px 14px;
		}

		.run-right {
			width: 100%;
			justify-content: space-between;
		}
	}
</style>
