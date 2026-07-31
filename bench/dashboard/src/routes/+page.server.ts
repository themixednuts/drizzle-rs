import type { PageServerLoad } from './$types';
import { overviewPageData } from '$lib/server/bench-data';
import { runServerEffect } from '$lib/server/effect';

export const load: PageServerLoad = ({ platform, url }) =>
	runServerEffect(
		overviewPageData({
			suite: url.searchParams.get('suite'),
			status: url.searchParams.get('status'),
		}),
		platform,
	);
