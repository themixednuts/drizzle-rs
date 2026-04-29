import type { PageServerLoad } from './$types';
import { runDetailPageData } from '$lib/server/bench-data';
import { runServerEffect } from '$lib/server/effect';

export const load: PageServerLoad = ({ platform, params }) =>
	runServerEffect(runDetailPageData(platform, params.run_id));
