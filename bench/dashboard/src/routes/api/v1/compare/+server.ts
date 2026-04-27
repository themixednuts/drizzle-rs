import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import type { operations, components } from '$lib/api-types';
import { extractCompareMetric, isCompareMetric } from '$lib/compare';
import { bucket, fetchManifest, fetchAllSummaries } from '$lib/r2';

type Metric = operations['compare']['parameters']['query']['metric'];
type CompareResponse = components['schemas']['compare'];

export const GET: RequestHandler = async ({ platform, url }) => {
	const baseId = url.searchParams.get('base');
	const headId = url.searchParams.get('head');
	const metric = url.searchParams.get('metric');

	if (!baseId || !headId || !metric) {
		error(400, 'Missing required parameters: base, head, metric');
	}
	if (!isCompareMetric(metric)) {
		error(400, `Invalid metric: ${metric}`);
	}

	const b = bucket(platform);

	const [baseManifest, headManifest] = await Promise.all([
		fetchManifest(b, baseId),
		fetchManifest(b, headId)
	]);

	const commonTargets = baseManifest.targets.filter((t) => headManifest.targets.includes(t));
	if (commonTargets.length === 0) error(400, 'No common targets between runs');

	const [baseSummaries, headSummaries] = await Promise.all([
		fetchAllSummaries(b, baseId, commonTargets),
		fetchAllSummaries(b, headId, commonTargets)
	]);

	const items = commonTargets
		.map((targetId) => {
			const bs = baseSummaries.find((s) => s.target_id === targetId);
			const hs = headSummaries.find((s) => s.target_id === targetId);
			if (!bs || !hs) return null;

			const baseVal = extractCompareMetric(bs, metric);
			const headVal = extractCompareMetric(hs, metric);
			const delta = headVal - baseVal;
			const deltaPct = baseVal !== 0 ? delta / baseVal : 0;

			return { target_id: targetId, base_value: baseVal, head_value: headVal, delta, delta_pct: deltaPct };
		})
		.filter((item) => item !== null);

	const body: CompareResponse = { metric: metric as Metric, base: baseId, head: headId, items };

	return json(body);
};
