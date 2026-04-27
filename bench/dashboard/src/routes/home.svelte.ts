import { page } from '$app/state';
import type { PageData } from './$types';

export class RunsPageState {
	#data: () => PageData;
	suite = $derived(page.url.searchParams.get('suite'));
	status = $derived(page.url.searchParams.get('status'));

	constructor(data: () => PageData) {
		this.#data = data;
	}

	get runs() {
		return this.#data().runs;
	}

	get suites() {
		return this.#data().suites;
	}

	get statuses() {
		return this.#data().statuses;
	}

	buildUrl = (suite: string | null, status: string | null): string => {
		const params = new URLSearchParams();
		if (suite) params.set('suite', suite);
		if (status) params.set('status', status);
		const query = params.toString();
		return '/' + (query ? '?' + query : '');
	};
}
