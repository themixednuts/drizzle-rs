<script lang="ts">
	import type { LatencyPercentiles } from '$lib/types';
	import { fmtLatency } from '$lib/format';

	interface Props {
		latency: LatencyPercentiles;
	}

	let { latency }: Props = $props();

	const tiers = $derived([
		{ label: 'avg', value: latency.avg },
		{ label: 'p90', value: latency.p90 },
		{ label: 'p95', value: latency.p95 },
		{ label: 'p99', value: latency.p99 },
		{ label: 'p999', value: latency.p999 }
	]);

	const maxVal = $derived(Math.max(...tiers.map((t) => t.value)) || 1);
</script>

<div class="bars">
	{#each tiers as tier}
		{@const pct = (tier.value / maxVal) * 100}
		<div class="bar-row">
			<span class="bar-label mono">{tier.label}</span>
			<div class="bar-track">
				<div
					class="bar-fill"
					style="width: {pct}%"
					class:warn={tier.label === 'p99' || tier.label === 'p999'}
				></div>
			</div>
			<span class="bar-value mono">{fmtLatency(tier.value)}</span>
		</div>
	{/each}
</div>

<style>
	.bars {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin-top: 8px;
	}

	.bar-row {
		display: grid;
		grid-template-columns: 36px 1fr 60px;
		align-items: center;
		gap: 8px;
	}

	.bar-label {
		font-size: 10px;
		color: var(--text-muted);
		text-align: right;
	}

	.bar-track {
		height: 6px;
		background: var(--bg-root);
		border-radius: 3px;
		overflow: hidden;
	}

	.bar-fill {
		height: 100%;
		border-radius: 3px;
		background: var(--cyan);
		opacity: 0.7;
		transition: width 0.4s ease;
	}

	.bar-fill.warn {
		background: var(--accent);
	}

	.bar-value {
		font-size: 11px;
		color: var(--text-secondary);
		text-align: right;
	}
</style>
