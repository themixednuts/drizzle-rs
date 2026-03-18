import type { PageServerLoad } from './$types';
import { bucket, fetchIndex, fetchManifest, fetchAllSummaries } from '$lib/r2';
import type { Summary, CompareItem } from '$lib/types';

type Metric = 'rps.avg' | 'rps.peak' | 'latency.avg' | 'latency.p95' | 'latency.p99' | 'cpu.avg' | 'cpu.peak' | 'err';

function extractMetric(s: Summary, metric: Metric): number {
	const p = s.primary;
	switch (metric) {
		case 'rps.avg': return p.rps.avg;
		case 'rps.peak': return p.rps.peak;
		case 'latency.avg': return p.latency.avg;
		case 'latency.p95': return p.latency.p95;
		case 'latency.p99': return p.latency.p99;
		case 'cpu.avg': return p.cpu.avg;
		case 'cpu.peak': return p.cpu.peak;
		case 'err': return p.err;
	}
}

export const load: PageServerLoad = async ({ platform, url }) => {
	const b = bucket(platform);
	const index = await fetchIndex(b);
	const runs = [...index.runs].sort((a, c) => c.run_id.localeCompare(a.run_id));

	const base = url.searchParams.get('base');
	const head = url.searchParams.get('head');
	const metric = url.searchParams.get('metric') ?? 'rps.avg';

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

			const baseVal = extractMetric(bs, metric as Metric);
			const headVal = extractMetric(hs, metric as Metric);
			const delta = headVal - baseVal;
			const deltaPct = baseVal !== 0 ? delta / baseVal : 0;

			return { target_id: targetId, base_value: baseVal, head_value: headVal, delta, delta_pct: deltaPct };
		})
		.filter((item): item is CompareItem => item !== null);

	return { runs, items };
};
