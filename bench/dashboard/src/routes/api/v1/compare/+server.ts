import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import type { operations, components } from '$lib/api-types';
import { bucket, fetchManifest, fetchAllSummaries } from '$lib/r2';
import type { Summary } from '$lib/types';

type Metric = operations['compare']['parameters']['query']['metric'];
type CompareResponse = components['schemas']['compare'];

const VALID_METRICS: Metric[] = [
	'rps.avg', 'rps.peak',
	'latency.avg', 'latency.p90', 'latency.p95', 'latency.p99', 'latency.p999',
	'cpu.avg', 'cpu.peak',
	'err'
];

function extractMetric(s: Summary, metric: Metric): number {
	const p = s.primary;
	switch (metric) {
		case 'rps.avg': return p.rps.avg;
		case 'rps.peak': return p.rps.peak;
		case 'latency.avg': return p.latency.avg;
		case 'latency.p90': return p.latency.p90;
		case 'latency.p95': return p.latency.p95;
		case 'latency.p99': return p.latency.p99;
		case 'latency.p999': return p.latency.p999;
		case 'cpu.avg': return p.cpu.avg;
		case 'cpu.peak': return p.cpu.peak;
		case 'err': return p.err;
	}
}

export const GET: RequestHandler = async ({ platform, url }) => {
	const baseId = url.searchParams.get('base');
	const headId = url.searchParams.get('head');
	const metric = url.searchParams.get('metric') as Metric | null;

	if (!baseId || !headId || !metric) {
		error(400, 'Missing required parameters: base, head, metric');
	}
	if (!VALID_METRICS.includes(metric)) {
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

			const baseVal = extractMetric(bs, metric);
			const headVal = extractMetric(hs, metric);
			const delta = headVal - baseVal;
			const deltaPct = baseVal !== 0 ? delta / baseVal : 0;

			return { target_id: targetId, base_value: baseVal, head_value: headVal, delta, delta_pct: deltaPct };
		})
		.filter((item) => item !== null);

	const body: CompareResponse = { metric, base: baseId, head: headId, items };

	return json(body);
};
