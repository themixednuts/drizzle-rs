import type { RequestHandler } from './$types';
import { compareApiData } from '$lib/server/bench-data';
import { runJsonEffect } from '$lib/server/effect';

export const GET: RequestHandler = ({ platform, url }) =>
	runJsonEffect(
		compareApiData(platform, {
			base: url.searchParams.get('base'),
			head: url.searchParams.get('head'),
			metric: url.searchParams.get('metric')
		})
	);
