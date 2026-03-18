import type { PageServerLoad } from './$types';
import { bucket, fetchIndex, fetchSummary } from '$lib/r2';
import type { TrendPoint } from '$lib/types';

export const load: PageServerLoad = async ({ platform, url }) => {
	const b = bucket(platform);
	const index = await fetchIndex(b);

	const suite = url.searchParams.get('suite');
	const target = url.searchParams.get('target');

	let runs = index.runs
		.filter((r) => r.status === 'success')
		.sort((a, c) => a.run_id.localeCompare(c.run_id));

	if (suite) runs = runs.filter((r) => r.suite === suite);

	const targets = [...new Set(runs.flatMap((r) => r.targets))].sort();
	const suites = [...new Set(index.runs.map((r) => r.suite))].sort();

	if (!target) {
		return { suites, targets, trends: [] as TrendPoint[] };
	}

	const relevantRuns = runs.filter((r) => r.targets.includes(target)).slice(-50);
	const summaries = await Promise.all(
		relevantRuns.map((r) => fetchSummary(b, r.run_id, target))
	);

	const trends: TrendPoint[] = relevantRuns
		.map((run, i) => {
			const s = summaries[i];
			if (!s) return null;
			return {
				run_id: run.run_id,
				start: run.start,
				git: run.git,
				rps_avg: s.primary.rps.avg,
				rps_peak: s.primary.rps.peak,
				latency_p95: s.primary.latency.p95,
				latency_p99: s.primary.latency.p99,
				cpu_avg: s.primary.cpu.avg,
				err: s.primary.err
			};
		})
		.filter((p): p is TrendPoint => p !== null);

	return { suites, targets, trends };
};
