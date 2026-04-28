import { Effect } from 'effect';
import {
	type BenchBucket,
	getBenchBucket,
	readAllSummaries,
	readIndex,
	readManifest,
	readSummary,
	readTimeseries
} from '$lib/r2';
import {
	extractCompareMetric,
	isCompareMetric,
	parseCompareMetric,
	type CompareMetric
} from '$lib/compare';
import type { CompareItem, Manifest, RunIndexEntry, Summary, Timeseries, TrendPoint } from '$lib/types';
import { failHttp } from './effect';

type MaybeFilter = string | null;

export interface LatestRunOverview {
	run: RunIndexEntry;
	manifest: Manifest;
	summaries: Summary[];
}

export function runsPageData(
	platform: App.Platform | undefined,
	filters: { suite: MaybeFilter; status: MaybeFilter }
) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		const index = yield* readIndex(bucket);

		let runs = [...index.runs];
		if (filters.suite) runs = runs.filter((run) => run.suite === filters.suite);
		if (filters.status) runs = runs.filter((run) => run.status === filters.status);
		runs.sort((a, b) => b.run_id.localeCompare(a.run_id));

		const latestRun = index.runs
			.filter((run) => run.status === 'success')
			.filter((run) => (filters.suite ? run.suite === filters.suite : true))
			.sort((a, b) => b.run_id.localeCompare(a.run_id))[0];

		let latest: LatestRunOverview | null = null;
		if (latestRun) {
			const manifest = yield* readManifest(bucket, latestRun.run_id);
			const summaries = yield* readAllSummaries(bucket, latestRun.run_id, manifest.targets);
			latest = { run: latestRun, manifest, summaries };
		}

		return {
			runs,
			latest,
			totalRuns: index.runs.length,
			totalTargets: new Set(index.runs.flatMap((run) => run.targets)).size,
			suites: [...new Set(index.runs.map((run) => run.suite))].sort(),
			statuses: [...new Set(index.runs.map((run) => run.status))].sort()
		};
	});
}

export function runDetailPageData(platform: App.Platform | undefined, runId: string) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		const manifest = yield* readManifest(bucket, runId);
		const summaries = yield* readAllSummaries(bucket, runId, manifest.targets);
		return { manifest, summaries };
	});
}

export function timeseriesData(
	platform: App.Platform | undefined,
	runId: string,
	targetId: string
) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		return yield* readTimeseries(bucket, runId, targetId);
	});
}

export function trendsPageData(
	platform: App.Platform | undefined,
	filters: { suite: MaybeFilter; target: MaybeFilter }
) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		const index = yield* readIndex(bucket);

		let runs = index.runs
			.filter((run) => run.status === 'success')
			.sort((a, b) => a.run_id.localeCompare(b.run_id));

		if (filters.suite) runs = runs.filter((run) => run.suite === filters.suite);

		const targets = [...new Set(runs.flatMap((run) => run.targets))].sort();
		const suites = [...new Set(index.runs.map((run) => run.suite))].sort();

		if (!filters.target) {
			return { suites, targets, trends: [] as TrendPoint[] };
		}

		const relevantRuns = runs.filter((run) => run.targets.includes(filters.target!)).slice(-50);
		const summaries = yield* Effect.forEach(
			relevantRuns,
			(run) => readSummary(bucket, run.run_id, filters.target!),
			{ concurrency: 'unbounded' }
		);

		const trends: TrendPoint[] = relevantRuns
			.map((run, index) => {
				const summary = summaries[index];
				if (!summary) return null;

				return {
					run_id: run.run_id,
					start: run.start,
					git: run.git,
					rps_avg: summary.primary.rps.avg,
					rps_peak: summary.primary.rps.peak,
					latency_p95: summary.primary.latency.p95,
					latency_p99: summary.primary.latency.p99,
					cpu_avg: summary.primary.cpu.avg,
					err: summary.primary.err
				};
			})
			.filter((point): point is TrendPoint => point !== null);

		return { suites, targets, trends };
	});
}

function compareItems(
	baseSummaries: readonly Summary[],
	headSummaries: readonly Summary[],
	commonTargets: readonly string[],
	metric: CompareMetric
): CompareItem[] {
	return commonTargets
		.map((targetId) => {
			const baseSummary = baseSummaries.find((summary) => summary.target_id === targetId);
			const headSummary = headSummaries.find((summary) => summary.target_id === targetId);
			if (!baseSummary || !headSummary) return null;

			const baseValue = extractCompareMetric(baseSummary, metric);
			const headValue = extractCompareMetric(headSummary, metric);
			const delta = headValue - baseValue;
			const deltaPct = baseValue !== 0 ? delta / baseValue : 0;

			return {
				target_id: targetId,
				base_value: baseValue,
				head_value: headValue,
				delta,
				delta_pct: deltaPct
			};
		})
		.filter((item): item is CompareItem => item !== null);
}

function compareRunItems(bucket: BenchBucket, base: string, head: string, metric: CompareMetric) {
	return Effect.gen(function* () {
		const [baseManifest, headManifest] = yield* Effect.all(
			[readManifest(bucket, base), readManifest(bucket, head)],
			{ concurrency: 'unbounded' }
		);

		const commonTargets = baseManifest.targets.filter((target) =>
			headManifest.targets.includes(target)
		);

		if (commonTargets.length === 0) return { commonTargets, items: [] as CompareItem[] };

		const [baseSummaries, headSummaries] = yield* Effect.all(
			[
				readAllSummaries(bucket, base, commonTargets),
				readAllSummaries(bucket, head, commonTargets)
			],
			{ concurrency: 'unbounded' }
		);

		return {
			commonTargets,
			items: compareItems(baseSummaries, headSummaries, commonTargets, metric)
		};
	});
}

export function comparePageData(
	platform: App.Platform | undefined,
	params: { base: MaybeFilter; head: MaybeFilter; metric: string | null }
) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		const index = yield* readIndex(bucket);
		const runs = [...index.runs].sort((a, b) => b.run_id.localeCompare(a.run_id));

		if (!params.base || !params.head) return { runs, items: null as CompareItem[] | null };

		const metric = parseCompareMetric(params.metric);
		const { items } = yield* compareRunItems(bucket, params.base, params.head, metric);
		return { runs, items };
	});
}

export function latestRunApiData(platform: App.Platform | undefined, suite: MaybeFilter) {
	return Effect.gen(function* () {
		if (!suite) return yield* failHttp(400, 'Missing required parameter: suite');

		const bucket = yield* getBenchBucket(platform);
		const index = yield* readIndex(bucket);
		const run = index.runs
			.filter((entry) => entry.suite === suite)
			.sort((a, b) => b.run_id.localeCompare(a.run_id))[0];

		if (!run) return yield* failHttp(404, `No run found for suite: ${suite}`);

		return {
			suite: run.suite,
			run_id: run.run_id,
			start: run.start,
			end: run.end,
			status: run.status
		};
	});
}

export function runManifestApiData(platform: App.Platform | undefined, runId: string) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		return yield* readManifest(bucket, runId);
	});
}

export function runSummaryApiData(
	platform: App.Platform | undefined,
	runId: string,
	targetsParam: MaybeFilter
) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		const manifest = yield* readManifest(bucket, runId);
		const targets = targetsParam
			? targetsParam.split(',').filter((target) => manifest.targets.includes(target))
			: manifest.targets;
		const items = yield* readAllSummaries(bucket, runId, targets);

		return { run_id: runId, items };
	});
}

function parseOptionalTime(name: string, value: MaybeFilter) {
	if (!value) return Effect.succeed(null);

	const time = Date.parse(value);
	return Number.isNaN(time)
		? failHttp(400, `Invalid ${name} timestamp: ${value}`)
		: Effect.succeed(time);
}

export function runTimeseriesApiData(
	platform: App.Platform | undefined,
	runId: string,
	params: { targets: MaybeFilter; from: MaybeFilter; to: MaybeFilter }
) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		const manifest = yield* readManifest(bucket, runId);
		const targets = params.targets
			? params.targets.split(',').filter((target) => manifest.targets.includes(target))
			: manifest.targets;
		const fromTime = yield* parseOptionalTime('from', params.from);
		const toTime = yield* parseOptionalTime('to', params.to);
		const results = yield* Effect.forEach(
			targets,
			(target) => readTimeseries(bucket, runId, target),
			{ concurrency: 'unbounded' }
		);

		const items: Timeseries[] = results
			.filter((timeseries): timeseries is Timeseries => timeseries !== null)
			.map((timeseries) => {
				if (!fromTime && !toTime) return timeseries;

				return {
					...timeseries,
					points: timeseries.points.filter((point) => {
						const time = Date.parse(point.time);
						if (fromTime && time < fromTime) return false;
						if (toTime && time > toTime) return false;
						return true;
					})
				};
			});

		return { run_id: runId, items };
	});
}

export function compareApiData(
	platform: App.Platform | undefined,
	params: { base: MaybeFilter; head: MaybeFilter; metric: MaybeFilter }
) {
	return Effect.gen(function* () {
		if (!params.base || !params.head || !params.metric) {
			return yield* failHttp(400, 'Missing required parameters: base, head, metric');
		}
		if (!isCompareMetric(params.metric)) {
			return yield* failHttp(400, `Invalid metric: ${params.metric}`);
		}

		const bucket = yield* getBenchBucket(platform);
		const { commonTargets, items } = yield* compareRunItems(bucket, params.base, params.head, params.metric);

		if (commonTargets.length === 0) {
			return yield* failHttp(400, 'No common targets between runs');
		}

		return { metric: params.metric, base: params.base, head: params.head, items };
	});
}
