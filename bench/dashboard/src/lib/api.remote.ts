import { query } from '$app/server';
import { getRequestEvent } from '$app/server';
import * as v from 'valibot';
import { bucket, fetchIndex, fetchManifest, fetchAllSummaries, fetchSummary, fetchTimeseries } from './r2';
import type { Summary, CompareItem, TrendPoint } from './types';

// --- Runs list ---

export const loadRuns = query(
	v.object({
		suite: v.nullable(v.string()),
		status: v.nullable(v.string())
	}),
	async ({ suite, status }) => {
		const b = bucket(getRequestEvent().platform);
		const index = await fetchIndex(b);

		let runs = index.runs;
		if (suite) runs = runs.filter((r) => r.suite === suite);
		if (status) runs = runs.filter((r) => r.status === status);
		runs.sort((a, c) => c.run_id.localeCompare(a.run_id));

		return {
			runs,
			suites: [...new Set(index.runs.map((r) => r.suite))].sort(),
			statuses: [...new Set(index.runs.map((r) => r.status))].sort()
		};
	}
);

// --- Run detail ---

export const loadRunDetail = query(
	v.object({ runId: v.string() }),
	async ({ runId }) => {
		const b = bucket(getRequestEvent().platform);
		const manifest = await fetchManifest(b, runId);
		const summaries = await fetchAllSummaries(b, runId, manifest.targets);
		return { manifest, summaries };
	}
);

// --- Timeseries (per-target, lazy) ---

export const loadTimeseries = query(
	v.object({ runId: v.string(), targetId: v.string() }),
	async ({ runId, targetId }) => {
		const b = bucket(getRequestEvent().platform);
		return fetchTimeseries(b, runId, targetId);
	}
);

// --- Trends ---

export const loadTrends = query(
	v.object({
		suite: v.nullable(v.string()),
		target: v.nullable(v.string())
	}),
	async ({ suite, target }) => {
		const b = bucket(getRequestEvent().platform);
		const index = await fetchIndex(b);

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
	}
);

// --- Compare ---

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

export const loadCompare = query(
	v.object({
		base: v.nullable(v.string()),
		head: v.nullable(v.string()),
		metric: v.string()
	}),
	async ({ base, head, metric }) => {
		const b = bucket(getRequestEvent().platform);
		const index = await fetchIndex(b);
		const runs = [...index.runs].sort((a, c) => c.run_id.localeCompare(a.run_id));

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
	}
);
