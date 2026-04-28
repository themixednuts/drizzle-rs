import { goto } from '$app/navigation';
import { page } from '$app/state';
import { isHigherBetterMetric, parseCompareMetric } from '$lib/compare';
import type { PageData } from './$types';

export class ComparePageState {
	#data: () => PageData;
	metric = $derived(parseCompareMetric(page.url.searchParams.get('metric')));

	constructor(data: () => PageData) {
		this.#data = data;
	}

	get cohorts() {
		return this.#data().cohorts;
	}

	get cohort() {
		return this.#data().cohort;
	}

	get cohortId() {
		return this.cohort?.id ?? page.url.searchParams.get('cohort');
	}

	get baseline() {
		return this.#data().baseline ?? page.url.searchParams.get('baseline');
	}

	get targets() {
		return this.#data().targets;
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
		if (this.metric.startsWith('mem')) return sign + abs.toFixed(1) + 'MB';
		if (this.metric === 'err') return sign + (abs * 100).toFixed(2) + '%';
		return sign + abs.toFixed(2);
	};

	barStyle = (deltaPct: number): string => {
		const side = deltaPct >= 0 ? 'left: 50%' : 'right: 50%';
		return `width: ${Math.min(Math.abs(deltaPct) * 100, 100)}%; ${side}`;
	};

	updateComparison = (event: Event): void => {
		const form = (event.currentTarget as HTMLSelectElement).form;
		if (!form) return;

		const data = new FormData(form);
		const params = new URLSearchParams();
		const cohort = data.get('cohort');
		const baseline = data.get('baseline');
		const metric = data.get('metric');
		if (typeof cohort === 'string' && cohort) params.set('cohort', cohort);
		if (typeof baseline === 'string' && baseline) params.set('baseline', baseline);
		params.set('metric', parseCompareMetric(typeof metric === 'string' ? metric : null));

		void goto('/compare?' + params.toString());
	};
}
