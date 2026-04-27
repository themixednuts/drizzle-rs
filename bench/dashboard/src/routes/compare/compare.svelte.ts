import { page } from '$app/state';
import { isHigherBetterMetric, parseCompareMetric } from '$lib/compare';
import type { PageData } from './$types';

export class ComparePageState {
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
