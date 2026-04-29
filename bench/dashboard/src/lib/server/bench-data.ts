import { Effect } from 'effect';
import {
	type BenchBucket,
	getBenchBucket,
	readAllSummaries,
	readIndex,
	readManifest,
	readTimeseries
} from '$lib/r2';
import {
	extractCompareCategorySortValue,
	extractCompareCategoryValues,
	extractCompareCategoryVariance,
	extractCompareMetric,
	isCompareMetric,
	isHigherBetterCategory,
	parseCompareCategory,
	type CompareMetric
} from '$lib/compare';
import { targetLabel } from '$lib/target-display';
import type {
	CompareItem,
	Manifest,
	RunCohort,
	RunIndexEntry,
	Summary,
	SummaryResult,
	TargetCompareItem,
	TargetMeta,
	TargetOption,
	Timeseries,
	TrendPoint
} from '$lib/types';
import { failHttp } from './effect';

type MaybeFilter = string | null;
const COHORT_GAP_MS = 2 * 60 * 60 * 1000;

export interface LatestRunOverview {
	cohort: RunCohort;
	manifest: Manifest;
	summaries: SummaryResult[];
}

function isSameCohort(left: RunIndexEntry, right: RunIndexEntry): boolean {
	return (
		left.suite === right.suite &&
		left.status === right.status &&
		left.class === right.class &&
		left.git === right.git
	);
}

function buildRunCohorts(runs: readonly RunIndexEntry[]): RunCohort[] {
	const cohorts: RunCohort[] = [];
	const sorted = [...runs].sort((a, b) => a.start.localeCompare(b.start));

	for (const run of sorted) {
		const startMs = Date.parse(run.start);
		const cohort = [...cohorts].reverse().find((candidate) => {
			const representative: RunIndexEntry = {
				run_id: candidate.representative_run_id,
				name: candidate.name,
				suite: candidate.suite,
				status: candidate.status,
				class: candidate.class,
				git: candidate.git,
				start: candidate.start,
				end: candidate.end,
				targets: candidate.targets
			};
			const previousEndMs = Date.parse(candidate.end);
			return isSameCohort(representative, run) && startMs - previousEndMs <= COHORT_GAP_MS;
		});

		if (!cohort) {
			cohorts.push({
				id: run.run_id,
				name: run.name,
				suite: run.suite,
				status: run.status,
				class: run.class,
				git: run.git,
				start: run.start,
				end: run.end,
				run_ids: [run.run_id],
				representative_run_id: run.run_id,
				targets: [...run.targets],
				result_count: run.targets.length
			});
			continue;
		}

		cohort.start = cohort.start < run.start ? cohort.start : run.start;
		cohort.end = cohort.end > run.end ? cohort.end : run.end;
		cohort.run_ids.push(run.run_id);
		cohort.representative_run_id =
			run.run_id > cohort.representative_run_id ? run.run_id : cohort.representative_run_id;
		cohort.targets = [...new Set([...cohort.targets, ...run.targets])].sort();
		cohort.result_count += run.targets.length;
	}

	return cohorts;
}

function resultKey(targetId: string, manifest: Manifest): string {
	return `${targetId}@${manifest.runner.os.toLowerCase()}`;
}

function targetMeta(manifest: Manifest, targetId: string): TargetMeta | undefined {
	return manifest.target_meta.find((target) => target.id === targetId);
}

function requireTargetMeta(manifest: Manifest, targetId: string): TargetMeta {
	const meta = targetMeta(manifest, targetId);
	if (!meta) {
		throw new Error(`manifest ${manifest.run_id} missing target_meta for ${targetId}`);
	}
	return meta;
}

function toSummaryResult(cohort: RunCohort, manifest: Manifest, summary: Summary): SummaryResult {
	const meta = requireTargetMeta(manifest, summary.target_id);
	return {
		...summary,
		group: meta.group ?? summary.group,
		cohort_id: cohort.id,
		target_key: resultKey(summary.target_id, manifest),
		target_name: meta.name,
		target_description: meta.description,
		target_meta: meta,
		runner_os: manifest.runner.os,
		runner_class: manifest.runner.class,
		runner_label: `${manifest.runner.os} / ${manifest.runner.class}`
	};
}

function targetOptions(results: readonly SummaryResult[]): TargetOption[] {
	const options = new Map<string, TargetOption>();
	for (const result of results) {
		options.set(result.target_key, {
			key: result.target_key,
			label: targetLabel(result),
			target_id: result.target_id,
			target_name: result.target_name,
			target_meta: result.target_meta,
			runner_os: result.runner_os
		});
	}
	return [...options.values()].sort((a, b) => a.label.localeCompare(b.label));
}

function resolveTargetKey(targets: readonly TargetOption[], value: string): string | null {
	const exact = targets.find((option) => option.key === value);
	if (exact) return exact.key;

	const matches = targets.filter((option) => option.target_id === value);
	return matches.length === 1 ? matches[0].key : null;
}

function readCohortSnapshot(bucket: BenchBucket, cohort: RunCohort) {
	return Effect.gen(function* () {
		const manifests = yield* Effect.forEach(
			cohort.run_ids,
			(runId) => readManifest(bucket, runId),
			{ concurrency: 'unbounded' }
		);

		const perRun = yield* Effect.forEach(
			manifests,
			(manifest) =>
				Effect.gen(function* () {
					const summaries = yield* readAllSummaries(bucket, manifest.run_id, manifest.targets);
					return summaries.map((summary) => toSummaryResult(cohort, manifest, summary));
				}),
			{ concurrency: 'unbounded' }
		);

		const summaries = perRun
			.flat()
			.sort((a, b) => b.primary.rps.avg - a.primary.rps.avg || a.target_key.localeCompare(b.target_key));
		const manifest =
			manifests.find((candidate) => candidate.run_id === cohort.representative_run_id) ??
			manifests[manifests.length - 1];
		if (!manifest) {
			throw new Error(`cohort ${cohort.id} has no manifests`);
		}

		return { cohort, manifest, summaries };
	});
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

		const cohorts = buildRunCohorts(runs).sort((a, b) => b.start.localeCompare(a.start));
		const allCohorts = buildRunCohorts(index.runs);
		const latestCohort = cohorts.find((cohort) => cohort.status === 'success');

		let latest: LatestRunOverview | null = null;
		if (latestCohort) {
			latest = yield* readCohortSnapshot(bucket, latestCohort);
		}

		return {
			runs,
			cohorts,
			latest,
			totalRuns: index.runs.length,
			totalCohorts: allCohorts.length,
			totalResults: index.runs.reduce((sum, run) => sum + run.targets.length, 0),
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

		const suites = [...new Set(index.runs.map((run) => run.suite))].sort();
		const cohorts = buildRunCohorts(runs).sort((a, b) => a.start.localeCompare(b.start));
		const snapshots = yield* Effect.forEach(
			cohorts.slice(-50),
			(cohort) => readCohortSnapshot(bucket, cohort),
			{ concurrency: 'unbounded' }
		);
		const targets = targetOptions(snapshots.flatMap((snapshot) => snapshot.summaries));

		if (!filters.target) {
			return { suites, targets, trends: [] as TrendPoint[] };
		}

		const target = resolveTargetKey(targets, filters.target);
		if (!target) {
			return { suites, targets, trends: [] as TrendPoint[] };
		}

		const trends: TrendPoint[] = snapshots
			.map((snapshot) => {
				const summary = snapshot.summaries.find((item) => item.target_key === target);
				if (!summary) return null;

				const point: TrendPoint = {
					cohort_id: snapshot.cohort.id,
					run_id: summary.run_id,
					start: snapshot.cohort.start,
					git: snapshot.cohort.git,
					rps_avg: summary.primary.rps.avg,
					rps_peak: summary.primary.rps.peak,
					latency_p95: summary.primary.latency.p95,
					latency_p99: summary.primary.latency.p99,
					cpu_avg: summary.primary.cpu.avg,
					err: summary.primary.err
				};
				if (summary.primary.mem) {
					point.mem_avg = summary.primary.mem.avg;
					point.mem_peak = summary.primary.mem.peak;
				}
				return point;
			})
			.filter((point): point is TrendPoint => point !== null);

		return { suites, targets, trends };
	});
}

function compareItems(
	baseSummaries: readonly Summary[],
	headSummaries: readonly Summary[],
	commonTargets: readonly string[],
	baseManifest: Manifest,
	metric: CompareMetric
): CompareItem[] {
	return commonTargets
		.map((targetId): CompareItem | null => {
			const baseSummary = baseSummaries.find((summary) => summary.target_id === targetId);
			const headSummary = headSummaries.find((summary) => summary.target_id === targetId);
			if (!baseSummary || !headSummary) return null;

			const baseValue = extractCompareMetric(baseSummary, metric);
			const headValue = extractCompareMetric(headSummary, metric);
			if (baseValue === null || headValue === null) return null;
			const delta = headValue - baseValue;
			const deltaPct = baseValue !== 0 ? delta / baseValue : 0;
			const meta = requireTargetMeta(baseManifest, targetId);

			return {
				target_key: targetId,
				target_id: targetId,
				target_name: meta.name,
				group: meta.group ?? baseSummary.group,
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
			items: compareItems(baseSummaries, headSummaries, commonTargets, baseManifest, metric)
		};
	});
}

export function comparePageData(
	platform: App.Platform | undefined,
	params: { cohort: MaybeFilter; metric: string | null }
) {
	return Effect.gen(function* () {
		const bucket = yield* getBenchBucket(platform);
		const index = yield* readIndex(bucket);
		const cohorts = buildRunCohorts(index.runs.filter((run) => run.status === 'success')).sort((a, b) =>
			b.start.localeCompare(a.start)
		);
		const category = parseCompareCategory(params.metric);
		const cohort = cohorts.find((item) => item.id === params.cohort) ?? cohorts[0] ?? null;

		if (!cohort) {
			return {
				cohorts,
				cohort: null,
				targets: [] as TargetOption[],
				items: null as TargetCompareItem[] | null
			};
		}

		const snapshot = yield* readCohortSnapshot(bucket, cohort);
		const comparable = snapshot.summaries
			.map((summary) => ({
				summary,
				sortValue: extractCompareCategorySortValue(summary, category),
				values: extractCompareCategoryValues(summary, category),
				variance: extractCompareCategoryVariance(summary, category)
			}))
			.filter(
				(
					item
				): item is {
					summary: SummaryResult;
					sortValue: number;
					values: NonNullable<ReturnType<typeof extractCompareCategoryValues>>;
					variance: NonNullable<ReturnType<typeof extractCompareCategoryVariance>>;
				} => item.sortValue !== null && item.values !== null && item.variance !== null
			);
		const items: TargetCompareItem[] = comparable
			.map(({ summary, sortValue, values, variance }) => {
				return {
					target_key: summary.target_key,
					target_id: summary.target_id,
					target_name: summary.target_name,
					target_description: summary.target_description,
					target_meta: summary.target_meta,
					group: summary.group,
					runner_os: summary.runner_os,
					values,
					sort_value: sortValue,
					variance,
					err: summary.primary.err
				};
			})
			.sort((a, b) => {
				const aBad = a.err > 0.005;
				const bBad = b.err > 0.005;
				if (aBad !== bBad) return aBad ? 1 : -1;
				return isHigherBetterCategory(category)
					? b.sort_value - a.sort_value
					: a.sort_value - b.sort_value;
			});

		return {
			cohorts,
			cohort,
			targets: targetOptions(comparable.map((item) => item.summary)),
			items
		};
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
