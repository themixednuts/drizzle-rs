import type { PageServerLoad } from './$types';
import { runDetailPageData } from '#lib/server/bench-data';
import { runServerEffect } from '#lib/server/effect';
import { parseChartMetric } from '#lib/metrics';

/**
 * `?metric=` is read here, not held in client state, so the metric tabs are ordinary links: with
 * scripting off a click navigates and the server renders that metric's charts.
 */
export const load: PageServerLoad = ({ platform, params, url }) =>
	runServerEffect(
		runDetailPageData(params.run_id, parseChartMetric(url.searchParams.get('metric'))),
		platform,
	);
