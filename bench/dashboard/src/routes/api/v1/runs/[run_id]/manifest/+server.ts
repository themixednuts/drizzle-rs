import type { RequestHandler } from './$types';
import { runManifestApiData } from '$lib/server/bench-data';
import { runJsonEffect } from '$lib/server/effect';

export const GET: RequestHandler = ({ platform, params }) =>
	runJsonEffect(runManifestApiData(params.run_id), platform);
