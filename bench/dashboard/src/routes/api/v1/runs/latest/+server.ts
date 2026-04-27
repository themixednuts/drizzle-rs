import type { RequestHandler } from './$types';
import { latestRunApiData } from '$lib/server/bench-data';
import { runJsonEffect } from '$lib/server/effect';

export const GET: RequestHandler = ({ platform, url }) =>
	runJsonEffect(latestRunApiData(platform, url.searchParams.get('suite')));
