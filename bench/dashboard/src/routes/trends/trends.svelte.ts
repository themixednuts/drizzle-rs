import { goto } from '$app/navigation';
import { page } from '$app/state';
import type { PageData } from './$types';

export class TrendsPageState {
	#data: () => PageData;
	suite = $derived(page.url.searchParams.get('suite'));
	target = $derived(page.url.searchParams.get('target'));

	constructor(data: () => PageData) {
		this.#data = data;
	}

	get suites() {
		return this.#data().suites;
	}

	get targets() {
		return this.#data().targets;
	}

	get targetKey() {
		const target = this.target;
		if (!target) return null;
		const exact = this.targets.find((option) => option.key === target);
		if (exact) return exact.key;
		const matches = this.targets.filter((option) => option.target_id === target);
		return matches.length === 1 ? matches[0].key : target;
	}

	get targetLabel() {
		const targetKey = this.targetKey;
		return this.targets.find((option) => option.key === targetKey)?.label ?? targetKey;
	}

	get trends() {
		return this.#data().trends;
	}

	get latest() {
		return this.trends.at(-1) ?? null;
	}

	get previous() {
		return this.trends.length > 1 ? this.trends[this.trends.length - 2] : null;
	}

	get reversedTrends() {
		return [...this.trends].reverse();
	}

	buildUrl = (suite: string | null, target: string | null): string => {
		const params = new URLSearchParams();
		if (suite) params.set('suite', suite);
		if (target) params.set('target', target);
		const qs = params.toString();
		return '/trends' + (qs ? '?' + qs : '');
	};

	selectTarget = (event: Event): void => {
		const value = (event.currentTarget as HTMLSelectElement).value;
		void goto(this.buildUrl(this.suite, value || null));
	};
}
