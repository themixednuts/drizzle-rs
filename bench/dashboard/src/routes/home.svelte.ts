import { page } from '$app/state';
import { boxWhiskerExtent, rpsBox } from '$lib/boxplot';
import { fmtCpu, fmtLatency, fmtPct, fmtRps, suiteLabel } from '$lib/format';
import {
	drizzleDelta,
	drizzleDeltaDirection,
	groupTargets,
	type DeltaDirection,
	type TargetGroup,
} from '$lib/leaderboard';
import { isDrizzleRsTarget, targetDisplay } from '$lib/target-display';
import type { FilterOption } from '$lib/components/FilterPills.svelte';
import type { Manifest, RunCohort, RunIndexEntry, SummaryResult } from '$lib/types';

interface RunsPageData {
	runs: RunIndexEntry[];
	cohorts: RunCohort[];
	warnings: string[];
	latest?: { cohort: RunCohort; manifest: Manifest; summaries: SummaryResult[] } | null;
	totalRuns: number;
	totalCohorts: number;
	totalResults: number;
	totalTargets: number;
	hasData: boolean;
	suites: string[];
	statuses: string[];
}

export interface LeaderboardRow {
	/**
	 * Render identity. `target_key` is deliberately shard-independent (it is what dedupes the
	 * target dropdowns), so two shards of the same OS running the same target share one — which
	 * makes it unusable as a keyed-each key on its own.
	 */
	id: string;
	summary: SummaryResult;
	/** Position inside its own section; null in sections that are not ranked. */
	rank: number | null;
	isBaseline: boolean;
	deltaText: string;
	deltaDirection: DeltaDirection;
	deltaTitle: string;
}

export interface LeaderboardSection {
	key: string;
	label: string;
	note: string | null;
	ranked: boolean;
	rows: LeaderboardRow[];
	baselineName: string | null;
	shards: { os: string; run_id: string }[];
	extent: ReturnType<typeof boxWhiskerExtent>;
}

function hasMaterialErrors(summary: SummaryResult): boolean {
	return summary.primary.err > 0.005;
}

function compareLeaderboard(a: SummaryResult, b: SummaryResult): number {
	const aBad = hasMaterialErrors(a);
	const bBad = hasMaterialErrors(b);
	if (aBad !== bBad) return aBad ? 1 : -1;
	return b.primary.rps.avg - a.primary.rps.avg;
}

export class RunsPageState {
	#data: () => RunsPageData;
	#basePath: string;
	query = $state('');
	hoverFamilyKey = $state<string | null>(null);
	suite = $derived(page.url.searchParams.get('suite'));
	status = $derived(page.url.searchParams.get('status'));

	constructor(data: () => RunsPageData, basePath = '/') {
		this.#data = data;
		this.#basePath = basePath;
	}

	get runs() {
		return this.#data().runs;
	}

	get cohorts() {
		return this.#data().cohorts;
	}

	get recentCohorts() {
		return this.cohorts.slice(0, 12);
	}

	get warnings() {
		return this.#data().warnings ?? [];
	}

	get hasData() {
		return this.#data().hasData;
	}

	get filteredCohorts() {
		const query = this.query.trim().toLowerCase();
		if (!query) return this.cohorts;
		return this.cohorts.filter((cohort) => {
			const text = [
				cohort.id,
				cohort.name,
				cohort.git,
				cohort.suite,
				cohort.status,
				cohort.class,
				...cohort.targets,
				...cohort.run_ids,
			]
				.join(' ')
				.toLowerCase();
			return text.includes(query);
		});
	}

	get latest() {
		return this.#data().latest ?? null;
	}

	get suites() {
		return this.#data().suites;
	}

	get statuses() {
		return this.#data().statuses;
	}

	get totalRuns() {
		return this.#data().totalRuns;
	}

	get totalCohorts() {
		return this.#data().totalCohorts;
	}

	get totalResults() {
		return this.#data().totalResults;
	}

	get totalTargets() {
		return this.#data().totalTargets;
	}

	get results() {
		return this.latest?.summaries ?? [];
	}

	/**
	 * Rows are grouped by database family instead of being poured into one RPS-sorted table:
	 * an embedded SQLite file, a TCP PostgreSQL connection and an in-process cache are not
	 * doing the same work, so a single ranking would imply a comparison that does not hold.
	 */
	sections: LeaderboardSection[] = $derived(
		groupTargets(this.results, compareLeaderboard).map((group) => this.#toSection(group)),
	);

	#toSection(group: TargetGroup<SummaryResult>): LeaderboardSection {
		const baseline = group.baseline;
		const baselineRps = baseline?.primary.rps.avg ?? null;
		const rows = group.rows.map((summary, index): LeaderboardRow => {
			const isBaseline = summary === baseline;
			return {
				id: `${summary.run_id}:${summary.target_key}`,
				summary,
				rank: group.ranked ? index + 1 : null,
				isBaseline,
				...this.#delta(summary, baselineRps, isBaseline),
			};
		});

		return {
			key: group.key,
			label: group.label,
			note: group.note,
			ranked: group.ranked,
			rows,
			baselineName: baseline ? targetDisplay(baseline).name : null,
			shards: group.shards,
			// Each family gets its own throughput scale; a shared one flattens the slower family
			// into an unreadable sliver.
			extent: boxWhiskerExtent(
				group.rows.map((summary) => rpsBox(summary)),
				group.rows.map((summary) => summary.primary.rps.avg),
			),
		};
	}

	#delta(
		summary: SummaryResult,
		baselineRps: number | null,
		isBaseline: boolean,
	): { deltaText: string; deltaDirection: DeltaDirection; deltaTitle: string } {
		if (baselineRps === null) {
			return {
				deltaText: '-',
				deltaDirection: 'flat',
				deltaTitle: 'no drizzle target in this database family to compare against',
			};
		}
		if (isBaseline) {
			return {
				deltaText: 'baseline',
				deltaDirection: 'flat',
				deltaTitle: 'the drizzle baseline row',
			};
		}
		if (hasMaterialErrors(summary)) {
			return {
				deltaText: 'errored',
				deltaDirection: 'flat',
				deltaTitle: 'error rate above 0.5%: throughput is not comparable',
			};
		}

		const delta = drizzleDelta(summary.primary.rps.avg, baselineRps, true);
		if (delta === null) {
			return { deltaText: '-', deltaDirection: 'flat', deltaTitle: 'not comparable' };
		}
		const pct = `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`;
		return {
			deltaText: pct,
			deltaDirection: drizzleDeltaDirection(delta),
			deltaTitle:
				delta >= 0
					? `drizzle does ${Math.abs(delta * 100).toFixed(1)}% more rps than this target`
					: `this target does ${Math.abs(delta * 100).toFixed(1)}% more rps than drizzle`,
		};
	}

	/**
	 * The target the KPI block describes. Never silently substitutes the fastest target for a
	 * drizzle one — when no drizzle target ran, the block says whose numbers it is showing.
	 */
	get kpiTarget(): { summary: SummaryResult; label: string; isOurs: boolean } | null {
		const results = this.results;
		if (results.length === 0) return null;

		const ours = [...results]
			.sort(compareLeaderboard)
			.find((summary) => isDrizzleRsTarget(summary));
		if (ours) {
			return { summary: ours, label: targetDisplay(ours).name, isOurs: true };
		}
		const fastest = [...results].sort(compareLeaderboard)[0];
		return { summary: fastest, label: `fastest: ${targetDisplay(fastest).name}`, isOurs: false };
	}

	get overviewMeta() {
		const cohort = this.latest?.cohort;
		if (!cohort)
			return `${this.totalCohorts} sets / ${this.totalResults} results / ${this.totalTargets} target ids`;
		const runner = this.latest!.manifest.runner;
		return `${this.totalCohorts} set / ${this.totalResults} results / ${this.totalTargets} target ids / ${runner.class} / ${runner.cores} cores`;
	}

	get filterMeta() {
		const latest = this.latest;
		if (!latest) return `${this.cohorts.length} matching sets`;

		const load = latest.manifest.load;
		const trials = latest.manifest.trials;
		return `${latest.cohort.result_count} results / ${latest.cohort.run_ids.length} shards / n=${trials.count} trials, ${trials.aggregate} across trials / ${load.duration_s}s / ${load.max_vus} max vus${load.pacing ? ` / ${load.pacing} pacing` : ''}`;
	}

	/**
	 * Labels say `median` wherever the number is the runner's cross-trial aggregate, which is a
	 * median even though the artifact key is spelled `avg`.
	 */
	get kpis() {
		const target = this.kpiTarget;
		if (!target) return [];

		const p = target.summary.primary;
		return [
			{
				label: 'rps median',
				value: fmtRps(p.rps.avg),
				detail: `peak ${fmtRps(p.rps.peak)}`,
				hint: 'median requests/second across trials',
			},
			{
				label: 'lat mean',
				value: fmtLatency(p.latency.avg),
				detail: 'median across trials',
				hint: "median across trials of each trial's mean latency",
			},
			{
				label: 'lat p95',
				value: fmtLatency(p.latency.p95),
				detail: 'median across trials',
				hint: 'median across trials of the 95th percentile',
			},
			{
				label: 'lat p99',
				value: fmtLatency(p.latency.p99),
				detail: 'median across trials',
				hint: 'median across trials of the 99th percentile',
			},
			{
				label: 'cpu median',
				value: fmtCpu(p.cpu.avg),
				detail: `peak core ${fmtCpu(p.cpu.peak)}`,
				hint: 'median across trials of mean-across-cores utilization; peak core is the highest single-core utilization',
			},
			p.mem
				? {
						label: 'mem median',
						value: `${p.mem.avg.toFixed(1)}MB`,
						detail: `peak ${p.mem.peak.toFixed(1)}MB`,
						hint: 'median resident memory across trials',
					}
				: {
						label: 'err',
						value: fmtPct(p.err),
						detail: 'error rate',
						hint: 'errored requests / total requests',
					},
		];
	}

	targetDisplay(summary: SummaryResult) {
		return targetDisplay(summary);
	}

	/**
	 * Hovering a row lifts every row from the same target family and recedes the rest, which is how
	 * you follow one ORM across drivers without re-sorting the table.
	 */
	rowEmphasis(summary: SummaryResult): 'none' | 'related' | 'dimmed' {
		if (!this.hoverFamilyKey) return 'none';
		return targetDisplay(summary).familyKey === this.hoverFamilyKey ? 'related' : 'dimmed';
	}

	throughputBox(summary: SummaryResult) {
		return rpsBox(summary);
	}

	throughputLabel(summary: SummaryResult): string {
		const box = this.throughputBox(summary);
		const median = box.median === null ? 'n/a' : fmtRps(box.median);
		if (box.spread === 'boxplot') {
			return `rps across trials / min ${fmtRps(box.min)} / q1 ${fmtRps(box.q1 as number)} / median ${median} / q3 ${fmtRps(box.q3 as number)} / max ${fmtRps(box.max)} / n=${box.samples}`;
		}
		if (box.spread === 'range') {
			return `rps across trials / min ${fmtRps(box.min)} / median ${median} / max ${fmtRps(box.max)} / n=${box.samples} / no quartiles recorded`;
		}
		return `rps ${median} / no per-trial spread recorded`;
	}

	throughputSummaryLabel(summary: SummaryResult): string {
		const box = this.throughputBox(summary);
		const median = box.median === null ? 'n/a' : fmtRps(box.median);
		if (box.spread === 'none') return `${median} / n=${box.samples}`;
		return `min ${fmtRps(box.min)} / med ${median} / max ${fmtRps(box.max)} / n=${box.samples}`;
	}

	/** Both filter rows are built here so `/` and `/runs` cannot drift apart. */
	suiteFilters: FilterOption[] = $derived([
		{ label: 'all', href: this.buildUrl(null, this.status), active: !this.suite },
		...this.suites.map((suite) => ({
			label: suiteLabel(suite),
			href: this.buildUrl(suite, this.status),
			active: this.suite === suite,
		})),
	]);

	statusFilters: FilterOption[] = $derived([
		{ label: 'all', href: this.buildUrl(this.suite, null), active: !this.status },
		...this.statuses.map((status) => ({
			label: status,
			href: this.buildUrl(this.suite, status),
			active: this.status === status,
		})),
	]);

	buildUrl(suite: string | null, status: string | null): string {
		const params = new URLSearchParams();
		if (suite) params.set('suite', suite);
		if (status) params.set('status', status);
		const query = params.toString();
		return this.#basePath + (query ? '?' + query : '');
	}

	search = (event: Event): void => {
		this.query = (event.currentTarget as HTMLInputElement).value;
	};

	hoverTarget = (summary: SummaryResult): void => {
		this.hoverFamilyKey = targetDisplay(summary).familyKey;
	};

	clearHover = (): void => {
		this.hoverFamilyKey = null;
	};
}
