import type { PageServerLoad } from './$types';
import { trendsPageData } from '$lib/server/bench-data';
import { runServerEffect } from '$lib/server/effect';

export const load: PageServerLoad = ({ platform, url }) =>
	runServerEffect(
		trendsPageData(platform, {
			suite: url.searchParams.get('suite'),
			target: url.searchParams.get('target')
		})
	);
