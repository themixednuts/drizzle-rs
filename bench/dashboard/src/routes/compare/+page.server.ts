import type { PageServerLoad } from './$types';
import { comparePageData } from '$lib/server/bench-data';
import { runServerEffect } from '$lib/server/effect';

export const load: PageServerLoad = ({ platform, url }) =>
	runServerEffect(
		comparePageData(platform, {
			base: url.searchParams.get('base'),
			head: url.searchParams.get('head'),
			metric: url.searchParams.get('metric')
		})
	);
