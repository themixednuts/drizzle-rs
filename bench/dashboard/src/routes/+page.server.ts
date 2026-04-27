import type { PageServerLoad } from './$types';
import { runsPageData } from '$lib/server/bench-data';
import { runServerEffect } from '$lib/server/effect';

export const load: PageServerLoad = ({ platform, url }) =>
	runServerEffect(
		runsPageData(platform, {
			suite: url.searchParams.get('suite'),
			status: url.searchParams.get('status')
		})
	);
