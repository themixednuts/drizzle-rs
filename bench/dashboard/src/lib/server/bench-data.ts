import { Effect } from 'effect';
import { BenchStore, type BenchStoreInterface } from './bench-store';
import {
	extractCompareCategorySortValue,
	extractCompareCategoryValues,
	extractCompareCategoryBox,
	extractCompareCategoryVariance,
	extractCompareMetric,
	isCompareMetric,
	isHigherBetterCategory,
	parseCompareCategory,
	type CompareMetric,
} from '#lib/compare';
import {
	fallbackTargetMeta,
	isDrizzleRsTarget,
	targetDisplay,
	targetLabel,
} from '#lib/target-display';
import { buildTargetChart, emptyTargetChart, type TargetChart } from '#lib/chart-series';
import { buildOverlay, type OverlayTarget } from '#lib/overlay';
import type { RepeatabilityGroup } from '#lib/repeatability';
import { cohortSearchText } from '#lib/cohort-search';
import type { MetricKey } from '#lib/metrics';
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
	TrendPoint,
} from '#lib/types';
import { failHttp } from './effect';

type MaybeFilter = string | null;

/**
 * Cloudflare Workers cap a request at 1000 subrequests. Every R2 `get` counts, so all fan-out
 * over cohorts/targets is bounded rather than `'unbounded'`.
 */
const R2_CONCURRENCY = 10;

/** How many cohorts of history the trends page walks. */
const TREND_COHORT_LIMIT = 50;

export interface LatestRunOverview {
	cohort: RunCohort;
	manifest: Manifest;
	summaries: SummaryResult[];
}

export interface CohortBuild {
	cohorts: RunCohort[];
	/** Non-fatal data problems surfaced to the UI instead of failing the whole page. */
	warnings: string[];
}

function mergeStatus(left: string, right: string): string {
	if (left === right) return left;
	if (left === 'failed' || right === 'failed') return 'failed';
	if (left === 'partial' || right === 'partial') return 'partial';
	if (left === 'canceled' || right === 'canceled') return 'canceled';
	return right;
}

/**
 * Group index entries into cohorts.
 *
 * A run whose suite/class/commit disagrees with the rest of its cohort used to throw, which
 * turned one malformed index entry into a 500 for every page. It now lands in its own
 * synthetic cohort and the mismatch is reported as a warning.
 */
function buildRunCohorts(runs: readonly RunIndexEntry[]): CohortBuild {
	const cohorts = new Map<string, RunCohort>();
	const warnings: string[] = [];
	const sorted = [...runs].sort((a, b) => a.start.localeCompare(b.start));

	for (const run of sorted) {
		// A run published before shards were grouped has no cohort id. Such a run is its own
		// cohort — which is what a pre-cohort artifact means — rather than every one of them
		// collapsing into a single `undefined` bucket.
		let key = run.cohort_id ?? run.run_id;
		let cohort = cohorts.get(key);

		if (
			cohort &&
			(cohort.suite !== run.suite || cohort.class !== run.class || cohort.git !== run.git)
		) {
			warnings.push(
				`run ${run.run_id} does not match cohort ${run.cohort_id} (${run.suite}/${run.class}/${run.git} vs ${cohort.suite}/${cohort.class}/${cohort.git}); listing it separately`,
			);
			key = `${key}#${run.suite}/${run.class}/${run.git}`;
			cohort = cohorts.get(key);
		}

		if (!cohort) {
			cohorts.set(key, {
				id: key,
				name: run.name ?? key,
				suite: run.suite,
				status: run.status,
				class: run.class,
				git: run.git,
				start: run.start,
				end: run.end,
				run_ids: [run.run_id],
				representative_run_id: run.run_id,
				targets: [...run.targets],
				result_count: run.targets.length,
			});
			continue;
		}

		cohort.start = cohort.start < run.start ? cohort.start : run.start;
		cohort.end = cohort.end > run.end ? cohort.end : run.end;
		cohort.status = mergeStatus(cohort.status, run.status);
		cohort.run_ids.push(run.run_id);
		cohort.representative_run_id =
			run.run_id > cohort.representative_run_id ? run.run_id : cohort.representative_run_id;
		cohort.targets = [...new Set([...cohort.targets, ...run.targets])].sort();
		cohort.result_count += run.targets.length;
	}

	return { cohorts: [...cohorts.values()], warnings };
}

function resultKey(targetId: string, manifest: Manifest): string {
	return `${targetId}@${manifest.runner.os.toLowerCase()}`;
}

function targetMeta(manifest: Manifest, targetId: string): TargetMeta | undefined {
	// `?.` is load-bearing: artifacts published before the runner emitted `target_meta` have no
	// array at all, and indexing into it threw a 500 on every page that listed their targets.
	return manifest.target_meta?.find((target) => target.id === targetId);
}

/**
 * A summary whose manifest has no matching `target_meta` entry is degraded, not fatal: the row
 * still renders with a placeholder marked `incomplete` so the rest of the page survives.
 */
export function resolveTargetMeta(
	manifest: Manifest,
	targetId: string,
	group?: string,
): TargetMeta {
	return targetMeta(manifest, targetId) ?? fallbackTargetMeta(targetId, group);
}

function toSummaryResult(cohortId: string, manifest: Manifest, summary: Summary): SummaryResult {
	const meta = resolveTargetMeta(manifest, summary.target_id, summary.group);
	return {
		...summary,
		group: meta.group ?? summary.group,
		cohort_id: cohortId,
		target_key: resultKey(summary.target_id, manifest),
		target_name: meta.name,
		target_description: meta.description,
		target_meta: meta,
		runner_os: manifest.runner.os,
		runner_class: manifest.runner.class,
		runner_label: `${manifest.runner.os} / ${manifest.runner.class}`,
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
			runner_os: result.runner_os,
		});
	}
	return [...options.values()].sort((a, b) => a.label.localeCompare(b.label));
}

function manifestTargetOptions(manifests: readonly Manifest[]): TargetOption[] {
	const options = new Map<string, TargetOption>();
	for (const manifest of manifests) {
		for (const targetId of manifest.targets) {
			const meta = resolveTargetMeta(manifest, targetId);
			const key = resultKey(targetId, manifest);
			const option: TargetOption = {
				key,
				label: '',
				target_id: targetId,
				target_name: meta.name,
				target_meta: meta,
				runner_os: manifest.runner.os,
			};
			option.label = targetLabel(option);
			options.set(key, option);
		}
	}
	return [...options.values()].sort((a, b) => a.label.localeCompare(b.label));
}

interface TargetSelection {
	targetId: string;
	/** Lowercased runner OS, or null when the selection did not pin one. */
	os: string | null;
	key: string;
}

function resolveTargetSelection(
	targets: readonly TargetOption[],
	value: string,
): TargetSelection | null {
	const exact = targets.find((option) => option.key === value);
	if (exact)
		return { targetId: exact.target_id, os: exact.runner_os.toLowerCase(), key: exact.key };

	const byId = targets.find((option) => option.target_id === value);
	if (byId) return { targetId: byId.target_id, os: byId.runner_os.toLowerCase(), key: byId.key };

	// The selection may name a target that only exists in older cohorts and is therefore
	// missing from the current dropdown; parse `<target_id>@<os>` directly.
	const at = value.lastIndexOf('@');
	if (at > 0) {
		return { targetId: value.slice(0, at), os: value.slice(at + 1).toLowerCase(), key: value };
	}
	return value ? { targetId: value, os: null, key: value } : null;
}

function readCohortSnapshot(store: BenchStoreInterface, cohort: RunCohort) {
	return Effect.gen(function* () {
		const manifests = yield* Effect.forEach(cohort.run_ids, (runId) => store.readManifest(runId), {
			concurrency: R2_CONCURRENCY,
		});

		const perRun = yield* Effect.forEach(
			manifests,
			(manifest) =>
				Effect.gen(function* () {
					const summaries = yield* store.readAllSummaries(manifest.run_id, manifest.targets);
					return summaries.map((summary) => toSummaryResult(cohort.id, manifest, summary));
				}),
			{ concurrency: R2_CONCURRENCY },
		);

		const summaries = perRun
			.flat()
			.sort(
				(a, b) => b.primary.rps.avg - a.primary.rps.avg || a.target_key.localeCompare(b.target_key),
			);
		const manifest =
			manifests.find((candidate) => candidate.run_id === cohort.representative_run_id) ??
			manifests[manifests.length - 1];
		if (!manifest) {
			return yield* failHttp(404, `Benchmark set ${cohort.id} has no run manifests`);
		}

		return { cohort, manifest, summaries };
	});
}

function indexTotals(index: { runs: RunIndexEntry[] }) {
	return {
		totalRuns: index.runs.length,
		totalResults: index.runs.reduce((sum, run) => sum + run.targets.length, 0),
		totalTargets: new Set(index.runs.flatMap((run) => run.targets)).size,
		suites: [...new Set(index.runs.map((run) => run.suite))].sort(),
		statuses: [...new Set(index.runs.map((run) => run.status))].sort(),
	};
}

/**
 * Index-only page data for `/runs`. Deliberately does *not* fetch a cohort snapshot — the runs
 * list renders none of it, and fetching it cost a manifest + summary read per target per shard.
 */
export const runsPageData = Effect.fn('BenchData.runsPage')(function* (filters: {
	suite: MaybeFilter;
	status: MaybeFilter;
	q?: MaybeFilter;
}) {
	const store = yield* BenchStore;
	const index = yield* store.readIndexOrEmpty;

	let runs = [...index.runs];
	if (filters.suite) runs = runs.filter((run) => run.suite === filters.suite);
	if (filters.status) runs = runs.filter((run) => run.status === filters.status);
	runs.sort((a, b) => b.run_id.localeCompare(a.run_id));

	const filtered = buildRunCohorts(runs);
	const all = buildRunCohorts(index.runs);

	// The free-text filter is applied server-side so the search box is a real GET form that works
	// with scripting off; the client keeps filtering as you type purely as an enhancement.
	const query = filters.q?.trim().toLowerCase() ?? '';
	const cohorts = query
		? filtered.cohorts.filter((cohort) => cohortSearchText(cohort).includes(query))
		: filtered.cohorts;

	return {
		runs,
		q: filters.q ?? '',
		cohorts: cohorts.sort((a, b) => b.start.localeCompare(a.start)),
		warnings: [...new Set([...filtered.warnings, ...all.warnings])],
		totalCohorts: all.cohorts.length,
		hasData: index.runs.length > 0,
		...indexTotals(index),
	};
});

/** `/runs` plus the latest successful cohort snapshot rendered by the overview leaderboard. */
export const overviewPageData = Effect.fn('BenchData.overviewPage')(function* (filters: {
	suite: MaybeFilter;
	status: MaybeFilter;
}) {
	const base = yield* runsPageData(filters);
	const latestCohort = base.cohorts.find((cohort) => cohort.status === 'success');
	if (!latestCohort) return { ...base, latest: null as LatestRunOverview | null };

	const store = yield* BenchStore;
	// A broken snapshot degrades the overview to "no leaderboard", it does not 500 the page.
	const snapshot = yield* Effect.result(readCohortSnapshot(store, latestCohort));
	if (snapshot._tag === 'Failure') {
		return {
			...base,
			warnings: [...base.warnings, snapshot.failure.message],
			latest: null as LatestRunOverview | null,
		};
	}

	return { ...base, latest: snapshot.success as LatestRunOverview };
});

/**
 * A target only offers the memory tab when the runner sampled memory for it, so a page-wide
 * `?metric=mem` degrades to throughput for the targets that have none rather than charting zeroes.
 */
function chartMetricFor(summary: Summary, metric: MetricKey): MetricKey {
	return metric === 'mem' && !summary.primary.mem ? 'rps' : metric;
}

export const runDetailPageData = Effect.fn('BenchData.runDetailPage')(function* (
	runId: string,
	metric: MetricKey,
) {
	const store = yield* BenchStore;
	const manifest = yield* store.readManifest(runId);
	const summaries = yield* store.readAllSummaries(runId, manifest.targets);

	const queries = manifest.queries ?? [];
	const trials = manifest.trials.count;

	// Every chart on the page is derived here so the server can render all of them. The timeseries
	// artifacts are ~800 KB each and never leave this function — only the few hundred plotted points
	// do. Each target's artifact is read exactly once and feeds both its own per-metric chart and
	// its line on the two overlay charts.
	const derived = yield* Effect.forEach(
		summaries,
		(summary) =>
			Effect.map(store.readTimeseries(runId, summary.target_id), (timeseries) => {
				const wanted = chartMetricFor(summary, metric);
				const display = targetDisplay(summary);
				return {
					summary,
					chart: timeseries
						? buildTargetChart(timeseries.points, queries, wanted, trials)
						: emptyTargetChart(wanted),
					overlay: {
						targetId: summary.target_id,
						name: display.name,
						isOurs: isDrizzleRsTarget(summary),
						points: timeseries?.points ?? [],
					} satisfies OverlayTarget,
				};
			}),
		{ concurrency: R2_CONCURRENCY },
	);

	const overlayTargets = derived.map((entry) => entry.overlay);

	return {
		manifest,
		summaries,
		metric,
		charts: Object.fromEntries(
			derived.map((entry) => [entry.summary.target_id, entry.chart]),
		) as Record<string, TargetChart>,
		overlays: {
			rps: buildOverlay(overlayTargets, 'rps', trials),
			latency: buildOverlay(overlayTargets, 'latency', trials),
		},
	};
});

/**
 * One target's chart, for the remote function that swaps metrics after hydration. Same builder as
 * the initial render, so an enhanced switch and a full navigation produce identical output.
 */
export const targetChartData = Effect.fn('BenchData.targetChart')(function* (
	runId: string,
	targetId: string,
	metric: MetricKey,
) {
	const store = yield* BenchStore;
	const manifest = yield* store.readManifest(runId);
	const timeseries = yield* store.readTimeseries(runId, targetId);
	return timeseries
		? buildTargetChart(timeseries.points, manifest.queries ?? [], metric, manifest.trials.count)
		: emptyTargetChart(metric);
});

/**
 * Repeatability: the same job, run on more than one machine.
 *
 * This is the page the honesty of every other page leans on. The ranking table puts libraries that
 * ran on different machines in one list, which is only defensible if a reader can go and see, in
 * concrete numbers, how much a machine change moves a result. So it takes the newest successful
 * set, finds the targets whose work was repeated across shards, and shows each machine's throughput
 * side by side.
 *
 * A target that only ever ran on one machine is excluded rather than shown with a single bar: one
 * measurement says nothing about repeatability, and drawing it as if it did would be the exact
 * failure this page exists to prevent.
 */
export const repeatabilityPageData = Effect.fn('BenchData.repeatabilityPage')(function* (filters: {
	suite: MaybeFilter;
}) {
	const store = yield* BenchStore;
	const index = yield* store.readIndexOrEmpty;

	let runs = index.runs.filter((run) => run.status === 'success');
	if (filters.suite) runs = runs.filter((run) => run.suite === filters.suite);

	const suites = [...new Set(index.runs.map((run) => run.suite))].sort();
	const built = buildRunCohorts(runs);
	const cohort = built.cohorts.sort((a, b) => b.start.localeCompare(a.start))[0];

	const empty = {
		suites,
		cohort: null as RunCohort | null,
		groups: [] as RepeatabilityGroup[],
		warnings: built.warnings,
	};
	if (!cohort) return empty;

	// A broken snapshot degrades this page to "nothing to show", it does not 500.
	const snapshot = yield* Effect.result(readCohortSnapshot(store, cohort));
	if (snapshot._tag === 'Failure') {
		return { ...empty, cohort, warnings: [...built.warnings, snapshot.failure.message] };
	}

	const byTarget = new Map<string, SummaryResult[]>();
	for (const summary of snapshot.success.summaries) {
		const bucket = byTarget.get(summary.target_id);
		if (bucket) bucket.push(summary);
		else byTarget.set(summary.target_id, [summary]);
	}

	const groups: RepeatabilityGroup[] = [];
	for (const [targetId, summaries] of byTarget) {
		// "Different machines" means distinct runner labels, not merely distinct shards: two shards
		// of the same OS are the same machine class and would overstate the spread.
		const machines = new Map<string, SummaryResult>();
		for (const summary of summaries) {
			const key = summary.runner_label || summary.runner_os;
			if (!machines.has(key)) machines.set(key, summary);
		}
		if (machines.size < 2) continue;

		const runsOnMachines = [...machines.entries()]
			.map(([label, summary]) => ({
				label,
				run_id: summary.run_id,
				os: summary.runner_os,
				runner_class: summary.runner_class,
				rps: summary.primary.rps.avg,
				p95: summary.primary.latency.p95,
			}))
			.sort((a, b) => b.rps - a.rps);

		const values = runsOnMachines.map((entry) => entry.rps);
		const display = targetDisplay(summaries[0]);
		groups.push({
			targetId,
			name: display.name,
			note: display.note,
			api: display.api,
			isOurs: isDrizzleRsTarget(summaries[0]),
			min: Math.min(...values),
			max: Math.max(...values),
			runs: runsOnMachines,
		});
	}

	groups.sort((a, b) => b.max - a.max);
	return { suites, cohort, groups, warnings: built.warnings };
});

/**
 * Trend history for one target.
 *
 * The previous implementation read every manifest and every summary of every shard of the last
 * 50 cohorts with unbounded concurrency — thousands of R2 gets, over the Workers subrequest
 * cap, and it did the work even when no target was selected. Now:
 *   - no selection  -> only the newest cohort's manifests, purely to populate the dropdown;
 *   - a selection   -> per cohort, only the shards whose *index entry* already lists the target,
 *                      then that shard's manifest plus the one summary object.
 */
export const trendsPageData = Effect.fn('BenchData.trendsPage')(function* (filters: {
	suite: MaybeFilter;
	target: MaybeFilter;
}) {
	const store = yield* BenchStore;
	const index = yield* store.readIndexOrEmpty;

	let runs = index.runs
		.filter((run) => run.status === 'success')
		.sort((a, b) => a.run_id.localeCompare(b.run_id));

	if (filters.suite) runs = runs.filter((run) => run.suite === filters.suite);

	const suites = [...new Set(index.runs.map((run) => run.suite))].sort();
	const built = buildRunCohorts(runs);
	const cohorts = built.cohorts
		.sort((a, b) => a.start.localeCompare(b.start))
		.slice(-TREND_COHORT_LIMIT);
	const runsById = new Map(runs.map((run) => [run.run_id, run]));

	const newest = cohorts.at(-1);
	const targets = newest
		? manifestTargetOptions(
				yield* Effect.forEach(newest.run_ids, (runId) => store.readManifest(runId), {
					concurrency: R2_CONCURRENCY,
				}),
			)
		: [];

	const empty = { suites, targets, trends: [] as TrendPoint[], warnings: built.warnings };
	if (!filters.target) return empty;

	const selection = resolveTargetSelection(targets, filters.target);
	if (!selection) return empty;

	const points = yield* Effect.forEach(
		cohorts,
		(cohort) => trendPointForCohort(store, cohort, runsById, selection),
		{ concurrency: R2_CONCURRENCY },
	);

	return {
		suites,
		targets,
		trends: points.filter((point): point is TrendPoint => point !== null),
		warnings: built.warnings,
	};
});

function trendPointForCohort(
	store: BenchStoreInterface,
	cohort: RunCohort,
	runsById: ReadonlyMap<string, RunIndexEntry>,
	selection: TargetSelection,
): Effect.Effect<TrendPoint | null, never> {
	return Effect.gen(function* () {
		// The index already records which targets each shard produced, so shards that cannot
		// contain the selected target are skipped without reading their manifest at all.
		const candidates = cohort.run_ids.filter((runId) =>
			runsById.get(runId)?.targets.includes(selection.targetId),
		);
		if (candidates.length === 0) return null;

		const manifests = yield* Effect.forEach(candidates, (runId) => store.readManifest(runId), {
			concurrency: R2_CONCURRENCY,
		});
		const manifest = selection.os
			? manifests.find((item) => item.runner.os.toLowerCase() === selection.os)
			: manifests[0];
		if (!manifest) return null;

		const summary = yield* store.readSummary(manifest.run_id, selection.targetId);
		if (!summary) return null;

		const point: TrendPoint = {
			cohort_id: cohort.id,
			run_id: manifest.run_id,
			start: cohort.start,
			git: cohort.git,
			rps_avg: summary.primary.rps.avg,
			rps_peak: summary.primary.rps.peak,
			latency_p95: summary.primary.latency.p95,
			latency_p99: summary.primary.latency.p99,
			cpu_avg: summary.primary.cpu.avg,
			err: summary.primary.err,
		};
		if (summary.primary.mem) {
			point.mem_avg = summary.primary.mem.avg;
			point.mem_peak = summary.primary.mem.peak;
		}
		return point;
	}).pipe(Effect.orElseSucceed(() => null));
}

function compareItems(
	baseSummaries: readonly Summary[],
	headSummaries: readonly Summary[],
	commonTargets: readonly string[],
	baseManifest: Manifest,
	metric: CompareMetric,
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
			const meta = resolveTargetMeta(baseManifest, targetId, baseSummary.group);

			return {
				target_key: targetId,
				target_id: targetId,
				target_name: meta.name,
				group: meta.group ?? baseSummary.group,
				base_value: baseValue,
				head_value: headValue,
				delta,
				delta_pct: deltaPct,
			};
		})
		.filter((item): item is CompareItem => item !== null);
}

function compareRunItems(
	store: BenchStoreInterface,
	base: string,
	head: string,
	metric: CompareMetric,
) {
	return Effect.gen(function* () {
		const [baseManifest, headManifest] = yield* Effect.all(
			[store.readManifest(base), store.readManifest(head)],
			{ concurrency: R2_CONCURRENCY },
		);

		const commonTargets = baseManifest.targets.filter((target) =>
			headManifest.targets.includes(target),
		);

		if (commonTargets.length === 0) return { commonTargets, items: [] as CompareItem[] };

		const [baseSummaries, headSummaries] = yield* Effect.all(
			[store.readAllSummaries(base, commonTargets), store.readAllSummaries(head, commonTargets)],
			{ concurrency: R2_CONCURRENCY },
		);

		return {
			commonTargets,
			items: compareItems(baseSummaries, headSummaries, commonTargets, baseManifest, metric),
		};
	});
}

export const comparePageData = Effect.fn('BenchData.comparePage')(function* (params: {
	cohort: MaybeFilter;
	metric: string | null;
}) {
	const store = yield* BenchStore;
	const index = yield* store.readIndexOrEmpty;
	const built = buildRunCohorts(index.runs.filter((run) => run.status === 'success'));
	const cohorts = built.cohorts.sort((a, b) => b.start.localeCompare(a.start));
	const category = parseCompareCategory(params.metric);
	const cohort = cohorts.find((item) => item.id === params.cohort) ?? cohorts[0] ?? null;

	if (!cohort) {
		return {
			cohorts,
			cohort: null,
			warnings: built.warnings,
			targets: [] as TargetOption[],
			items: null as TargetCompareItem[] | null,
		};
	}

	const snapshot = yield* readCohortSnapshot(store, cohort);
	const comparable = snapshot.summaries
		.map((summary) => ({
			summary,
			sortValue: extractCompareCategorySortValue(summary, category),
			values: extractCompareCategoryValues(summary, category),
			variance: extractCompareCategoryVariance(summary, category),
			box: extractCompareCategoryBox(summary, category),
		}))
		.filter(
			(
				item,
			): item is {
				summary: SummaryResult;
				sortValue: number;
				values: NonNullable<ReturnType<typeof extractCompareCategoryValues>>;
				variance: NonNullable<ReturnType<typeof extractCompareCategoryVariance>>;
				box: NonNullable<ReturnType<typeof extractCompareCategoryBox>>;
			} =>
				item.sortValue !== null &&
				item.values !== null &&
				item.variance !== null &&
				item.box !== null,
		);
	const items: TargetCompareItem[] = comparable
		.map(({ summary, sortValue, values, variance, box }) => {
			return {
				target_key: summary.target_key,
				target_id: summary.target_id,
				target_name: summary.target_name,
				target_description: summary.target_description,
				target_meta: summary.target_meta,
				group: summary.group,
				run_id: summary.run_id,
				runner_os: summary.runner_os,
				// Drop the column `hint` here: it is static per category and the page reads it
				// from `compareCategoryColumns`, so shipping a copy per cell is dead payload.
				values: values.map(({ key, label, value }) => ({ key, label, value })),
				sort_value: sortValue,
				variance,
				box,
				err: summary.primary.err,
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
		warnings: built.warnings,
		targets: targetOptions(comparable.map((item) => item.summary)),
		items,
	};
});

export const latestRunApiData = Effect.fn('BenchData.latestRunApi')(function* (suite: MaybeFilter) {
	if (!suite) return yield* failHttp(400, 'Missing required parameter: suite');

	const store = yield* BenchStore;
	const index = yield* store.readIndex;
	const run = index.runs
		.filter((entry) => entry.suite === suite)
		.sort((a, b) => b.run_id.localeCompare(a.run_id))[0];

	if (!run) return yield* failHttp(404, `No run found for suite: ${suite}`);

	return {
		suite: run.suite,
		run_id: run.run_id,
		cohort_id: run.cohort_id,
		start: run.start,
		end: run.end,
		status: run.status,
	};
});

export const runManifestApiData = Effect.fn('BenchData.runManifestApi')(function* (runId: string) {
	const store = yield* BenchStore;
	return yield* store.readManifest(runId);
});

export const runSummaryApiData = Effect.fn('BenchData.runSummaryApi')(function* (
	runId: string,
	targetsParam: MaybeFilter,
) {
	const store = yield* BenchStore;
	const manifest = yield* store.readManifest(runId);
	const targets = targetsParam
		? targetsParam.split(',').filter((target) => manifest.targets.includes(target))
		: manifest.targets;
	const items = yield* store.readAllSummaries(runId, targets);

	return { run_id: runId, items };
});

function parseOptionalTime(name: string, value: MaybeFilter) {
	if (!value) return Effect.succeed(null);

	const time = Date.parse(value);
	return Number.isNaN(time)
		? failHttp(400, `Invalid ${name} timestamp: ${value}`)
		: Effect.succeed(time);
}

export const runTimeseriesApiData = Effect.fn('BenchData.runTimeseriesApi')(function* (
	runId: string,
	params: { targets: MaybeFilter; from: MaybeFilter; to: MaybeFilter },
) {
	const store = yield* BenchStore;
	const manifest = yield* store.readManifest(runId);
	const targets = params.targets
		? params.targets.split(',').filter((target) => manifest.targets.includes(target))
		: manifest.targets;
	const fromTime = yield* parseOptionalTime('from', params.from);
	const toTime = yield* parseOptionalTime('to', params.to);
	const results = yield* Effect.forEach(targets, (target) => store.readTimeseries(runId, target), {
		concurrency: R2_CONCURRENCY,
	});

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
				}),
			};
		});

	return { run_id: runId, items };
});

export const compareApiData = Effect.fn('BenchData.compareApi')(function* (params: {
	base: MaybeFilter;
	head: MaybeFilter;
	metric: MaybeFilter;
}) {
	if (!params.base || !params.head || !params.metric) {
		return yield* failHttp(400, 'Missing required parameters: base, head, metric');
	}
	if (!isCompareMetric(params.metric)) {
		return yield* failHttp(400, `Invalid metric: ${params.metric}`);
	}

	const store = yield* BenchStore;
	const { commonTargets, items } = yield* compareRunItems(
		store,
		params.base,
		params.head,
		params.metric,
	);

	if (commonTargets.length === 0) {
		return yield* failHttp(400, 'No common targets between runs');
	}

	return { metric: params.metric, base: params.base, head: params.head, items };
});
