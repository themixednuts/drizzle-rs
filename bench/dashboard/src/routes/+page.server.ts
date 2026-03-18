import type { PageServerLoad } from './$types';
import { bucket, fetchIndex } from '$lib/r2';

export const load: PageServerLoad = async ({ platform, url }) => {
	const b = bucket(platform);
	const index = await fetchIndex(b);

	const suite = url.searchParams.get('suite');
	const status = url.searchParams.get('status');

	let runs = index.runs;
	if (suite) runs = runs.filter((r) => r.suite === suite);
	if (status) runs = runs.filter((r) => r.status === status);
	runs.sort((a, c) => c.run_id.localeCompare(a.run_id));

	return {
		runs,
		suites: [...new Set(index.runs.map((r) => r.suite))].sort(),
		statuses: [...new Set(index.runs.map((r) => r.status))].sort()
	};
};
