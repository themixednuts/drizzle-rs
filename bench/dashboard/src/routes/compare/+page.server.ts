import type { PageServerLoad } from './$types';
import { comparePageData } from '$lib/server/bench-data';
import { runServerEffect } from '$lib/server/effect';

export const load: PageServerLoad = ({ platform, url }) =>
	runServerEffect(
		comparePageData(platform, {
			cohort: url.searchParams.get('cohort'),
			metric: url.searchParams.get('metric')
		})
	);
