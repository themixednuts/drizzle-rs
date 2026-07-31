import type { RequestHandler } from './$types';
import { runTimeseriesApiData } from '$lib/server/bench-data';
import { runJsonEffect } from '$lib/server/effect';

export const GET: RequestHandler = ({ platform, params, url }) =>
	runJsonEffect(
		runTimeseriesApiData(params.run_id, {
			targets: url.searchParams.get('targets'),
			from: url.searchParams.get('from'),
			to: url.searchParams.get('to'),
		}),
		platform,
	);
