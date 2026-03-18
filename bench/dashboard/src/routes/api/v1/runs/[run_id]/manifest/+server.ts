import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { bucket, fetchManifest } from '$lib/r2';

export const GET: RequestHandler = async ({ platform, params }) => {
	const b = bucket(platform);
	const manifest = await fetchManifest(b, params.run_id);
	return json(manifest);
};
