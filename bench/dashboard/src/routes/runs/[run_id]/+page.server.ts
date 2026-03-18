import type { PageServerLoad } from './$types';
import { bucket, fetchManifest, fetchAllSummaries } from '$lib/r2';

export const load: PageServerLoad = async ({ platform, params }) => {
	const b = bucket(platform);
	const manifest = await fetchManifest(b, params.run_id);
	const summaries = await fetchAllSummaries(b, params.run_id, manifest.targets);
	return { manifest, summaries };
};
