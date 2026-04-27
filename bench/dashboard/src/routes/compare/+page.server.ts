import type { PageServerLoad } from './$types';
import { bucket, fetchIndex, fetchManifest, fetchAllSummaries } from '$lib/r2';
import { extractCompareMetric, parseCompareMetric } from '$lib/compare';
import type { CompareItem } from '$lib/types';

export const load: PageServerLoad = async ({ platform, url }) => {
	const b = bucket(platform);
	const index = await fetchIndex(b);
	const runs = [...index.runs].sort((a, c) => c.run_id.localeCompare(a.run_id));

	const base = url.searchParams.get('base');
	const head = url.searchParams.get('head');
	const metric = parseCompareMetric(url.searchParams.get('metric'));

	if (!base || !head) return { runs, items: null as CompareItem[] | null };

	const [baseManifest, headManifest] = await Promise.all([
		fetchManifest(b, base),
		fetchManifest(b, head)
	]);

	const commonTargets = baseManifest.targets.filter((t) => headManifest.targets.includes(t));
	if (commonTargets.length === 0) return { runs, items: [] };

	const [baseSummaries, headSummaries] = await Promise.all([
		fetchAllSummaries(b, base, commonTargets),
		fetchAllSummaries(b, head, commonTargets)
	]);

	const items: CompareItem[] = commonTargets
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
		.filter((item): item is CompareItem => item !== null);

	return { runs, items };
};
