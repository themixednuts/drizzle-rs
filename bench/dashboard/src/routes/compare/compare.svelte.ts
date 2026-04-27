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
		const sign = value < 0 ? '-' : '';
		const abs = Math.abs(value);
		if (this.metric.startsWith('rps')) {
			if (abs >= 1_000_000) return sign + (abs / 1_000_000).toFixed(1) + 'M';
			if (abs >= 1_000) return sign + (abs / 1_000).toFixed(1) + 'k';
			return sign + abs.toFixed(0);
		}
		if (this.metric.startsWith('latency')) {
			if (abs >= 1_000) return sign + (abs / 1_000).toFixed(2) + 's';
			if (abs >= 1) return sign + abs.toFixed(1) + 'ms';
			return sign + (abs * 1_000).toFixed(0) + 'us';
		}
		if (this.metric.startsWith('cpu')) return sign + abs.toFixed(1) + '%';
		if (this.metric === 'err') return sign + (abs * 100).toFixed(2) + '%';
		return sign + abs.toFixed(2);
	};

	barStyle = (deltaPct: number): string => {
		const side = deltaPct >= 0 ? 'left: 50%' : 'right: 50%';
		return `width: ${Math.min(Math.abs(deltaPct) * 100, 100)}%; ${side}`;
	};
}
