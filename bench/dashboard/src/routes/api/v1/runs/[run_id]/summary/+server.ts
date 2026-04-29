import type { RequestHandler } from './$types';
import { runSummaryApiData } from '$lib/server/bench-data';
import { runJsonEffect } from '$lib/server/effect';

export const GET: RequestHandler = ({ platform, params, url }) =>
	runJsonEffect(runSummaryApiData(platform, params.run_id, url.searchParams.get('targets')));
