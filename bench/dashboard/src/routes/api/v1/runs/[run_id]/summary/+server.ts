import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { bucket, fetchManifest, fetchAllSummaries } from '$lib/r2';

export const GET: RequestHandler = async ({ platform, params, url }) => {
	const b = bucket(platform);
	const manifest = await fetchManifest(b, params.run_id);

	const targetsParam = url.searchParams.get('targets');
	const targets = targetsParam
		? targetsParam.split(',').filter((t) => manifest.targets.includes(t))
		: manifest.targets;

	const items = await fetchAllSummaries(b, params.run_id, targets);

	return json({ run_id: params.run_id, items });
};
