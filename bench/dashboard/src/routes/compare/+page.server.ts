import type { PageServerLoad } from './$types';
import { comparePageData } from '$lib/server/bench-data';
import { runServerEffect } from '$lib/server/effect';

export const load: PageServerLoad = ({ platform, url }) =>
	runServerEffect(
		comparePageData(platform, {
			cohort: url.searchParams.get('cohort'),
			baseline: url.searchParams.get('baseline'),
			metric: url.searchParams.get('metric')
		})
	);
