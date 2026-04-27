<script lang="ts">
	import type { LatencyPercentiles } from '$lib/types';
	import { fmtLatency } from '$lib/format';
	import { LatencyBarsState } from './latency-bars.svelte';

	interface Props {
		latency: LatencyPercentiles;
	}

	let { latency }: Props = $props();
	const view = new LatencyBarsState(() => latency);
</script>

<div class="bars">
	{#each view.tiers as tier}
		{@const pct = (tier.value / view.maxValue) * 100}
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
		background: var(--bg-2);
		border-radius: 3px;
		overflow: hidden;
	}

	.bar-fill {
		height: 100%;
		border-radius: 3px;
		background: var(--ink-4);
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
