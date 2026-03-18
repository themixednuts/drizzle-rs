import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import type { components } from '$lib/api-types';
import { bucket, fetchIndex } from '$lib/r2';

type LatestRun = components['schemas']['latest_run'];

export const GET: RequestHandler = async ({ platform, url }) => {
	const suite = url.searchParams.get('suite');
	if (!suite) error(400, 'Missing required parameter: suite');

	const b = bucket(platform);
	const index = await fetchIndex(b);

	const run = index.runs
		.filter((r) => r.suite === suite)
		.sort((a, c) => c.run_id.localeCompare(a.run_id))[0];

	if (!run) error(404, `No run found for suite: ${suite}`);

	const body: LatestRun = {
		suite: run.suite,
		run_id: run.run_id,
		start: run.start,
		end: run.end,
		status: run.status
	};

	return json(body);
};
