import type { LatencyPercentiles } from '$lib/types';

export class LatencyBarsState {
	#latency: () => LatencyPercentiles;
	tiers = $derived([
		{ label: 'avg', value: this.latency.avg },
		{ label: 'p90', value: this.latency.p90 },
		{ label: 'p95', value: this.latency.p95 },
		{ label: 'p99', value: this.latency.p99 },
		{ label: 'p999', value: this.latency.p999 }
	]);
	maxValue = $derived(Math.max(...this.tiers.map((tier) => tier.value)) || 1);

	constructor(latency: () => LatencyPercentiles) {
		this.#latency = latency;
	}

	get latency() {
		return this.#latency();
	}
}
