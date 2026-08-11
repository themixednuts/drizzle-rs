import { page } from '$app/state';
import { rpsBox } from '#lib/boxplot';
import { fmtCpu, fmtDate, fmtLatency, fmtPct, fmtRps, shortHash, suiteLabel } from '#lib/format';
import {
	baselinesByFamily,
	deltaDirection,
	deltaSentence,
	rowDelta,
	type DeltaDirection,
} from '#lib/leaderboard';
import {
	DB_PROFILE_ORDER,
	dbProfile,
	dbProfileDetail,
	dbShortLabel,
	familyLabel,
	isDrizzleRsTarget,
	isInProcessCache,
	targetDisplay,
	targetFamily,
	type DbProfile,
} from '#lib/target-display';
import { cohortSearchText } from '#lib/cohort-search';
import { summarizeAll, type QualitativeNote } from '#lib/qualitative';
import { anyMeasured, capacity, compareCapacity, type CapacityView } from '#lib/saturation';
import { harnessRows, type HarnessRow } from '#lib/harness';
import { osScopes, type OsScope } from '#lib/os';
import type { FilterOption } from '#lib/components/FilterPills.svelte';
import {
	ordinal,
	parseRankingSort,
	SORT_LABELS,
	type DbVerdict,
	type RankingRow,
	type RankingSort,
} from '#lib/ranking';
import type { HarnessFamily, Manifest, RunCohort, RunIndexEntry, SummaryResult } from '#lib/types';

interface RunsPageData {
	runs: RunIndexEntry[];
	q?: string;
	cohorts: RunCohort[];
	warnings: string[];
	latest?: {
		cohort: RunCohort;
		manifest: Manifest;
		summaries: SummaryResult[];
		/** Merged across every shard of the set — see `mergeHarness`. */
		harness: HarnessFamily[];
	} | null;
	totalRuns: number;
	totalCohorts: number;
	totalResults: number;
	totalTargets: number;
	hasData: boolean;
	suites: string[];
	statuses: string[];
}

function hasMaterialErrors(summary: SummaryResult): boolean {
	return summary.primary.err > 0.005;
}

/** Short names for the ranking's database column — the long forms are in `dbProfileLabel`. */
function dbLabel(profile: DbProfile): string {
	return dbShortLabel(profile);
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
	/**
	 * Seeded from `?q=`, which the server has already filtered on. Typing refines further without a
	 * navigation; with scripting off the form submit does the same work server-side.
	 */
	query = $state('');
	hoverFamilyKey = $state<string | null>(null);
	suite = $derived(page.url.searchParams.get('suite'));
	status = $derived(page.url.searchParams.get('status'));
	/** Ranking view state, all in the URL so a scoped ranking is a shareable address. */
	db = $derived(page.url.searchParams.get('db'));
	/**
	 * Which operating system the ranking is showing.
	 *
	 * Unlike `db`, this has no "All": a rank across operating systems is not a comparison, it is two
	 * comparisons stacked. Resolved against the scopes this set actually has, so a stale `?os=` from
	 * an older set lands on a real scope instead of an empty table.
	 */
	os = $derived(
		this.osScopes.find((scope) => scope.os === page.url.searchParams.get('os'))?.os ??
			this.defaultOs,
	);
	sort = $derived(
		parseRankingSort(page.url.searchParams.get('sort'), this.defaultSort, this.availableSorts),
	);

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
		return this.cohorts.filter((cohort) => cohortSearchText(cohort).includes(query));
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

	/** The search term the server rendered for, used to seed the input and the client filter. */
	get serverQuery(): string {
		return this.#data().q ?? '';
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

	/** Every operating system this set produced rows on, in a fixed order. */
	get osScopes(): OsScope[] {
		return osScopes(this.results);
	}

	/**
	 * The scope shown when the URL does not name one: the platform with the most targets.
	 *
	 * Not a hardcoded "linux". The default should be wherever the set has the most to say, and on a
	 * run where linux failed and the others did not, hardcoding it would open the page on an empty
	 * table. `osScopes` is already ordered, so ties resolve to Linux, then macOS, then Windows.
	 */
	get defaultOs(): string | null {
		const scopes = this.osScopes;
		if (scopes.length === 0) return null;
		return scopes.reduce((best, scope) => (scope.count > best.count ? scope : best)).os;
	}

	/** The scope currently on screen, with its provenance. Null only when the set has no rows. */
	get osScope(): OsScope | null {
		return this.osScopes.find((scope) => scope.os === this.os) ?? null;
	}

	/**
	 * The rows the ranking is computed over: one operating system's worth.
	 *
	 * Everything downstream — rank, bar scale, "vs drizzle-rs" baseline, whether this set measured
	 * capacity at all — reads this rather than `results`, because each of those is a within-scope
	 * claim. A baseline picked across operating systems would measure two machines and print the
	 * answer under a library's name.
	 */
	get #scopedResults(): SummaryResult[] {
		if (!this.os) return this.results;
		return this.results.filter((summary) => summary.runner_os === this.os);
	}

	/**
	 * The ranking: every target in the set, in one order, across every database.
	 *
	 * There are no family bands. Rank runs `01..N` over the whole list, the bar is scaled to the
	 * fastest row on screen, and SQLite sits directly above PostgreSQL if that is where the numbers
	 * put it — because whether an embedded engine beats a TCP one at this workload *is* part of the
	 * comparison, not an artefact to be partitioned away. Banding it also had a cost nobody wanted:
	 * the TypeScript and Prisma rows lived in a PostgreSQL band three screens down and readers
	 * reported simply not finding them.
	 *
	 * Everything the bands carried is still on the page, attached to rows instead of to layout: the
	 * `database` column (with the family's description on its tooltip), the OS badge, an in-process
	 * cache saying so in words on its own note line, the per-database "vs drizzle-rs" delta inside
	 * each row, and the footnote under the table pointing at Repeatability and Method.
	 */
	get rankingRows(): RankingRow[] {
		const rows = this.#scopedResults.filter(
			(summary) => !this.db || dbProfile(summary) === this.db,
		);
		const ordered = [...rows].sort(this.#comparator);

		// Scaled to the rows on screen, not to every row that exists. With "all databases" selected
		// the in-memory-cache target is several times faster than anything doing real per-request
		// work, and an absolute scale would squash every other bar into an unreadable stub. The
		// number beside each bar is always the real one, so the bar is a shape cue and never the
		// source of a value.
		const baselines = this.#baselines;
		const peak = Math.max(1, ...ordered.map((summary) => summary.primary.rps.avg));
		const peakLatency = Math.max(1, ...ordered.map((summary) => summary.primary.latency.p95));
		const peakCapacity = Math.max(1, ...ordered.map((summary) => this.capacity(summary).tierValue));

		// Ranks are handed out only to rows the sorted column actually measured, and they run
		// 01..N over those. Under `sort=capacity` that means the unmeasured rows carry no number
		// at all rather than a number that would read as a placement.
		let rank = 0;

		return ordered.map((summary) => {
			const view = this.capacity(summary);
			const ranked = this.sort !== 'capacity' || view.rankable;
			if (ranked) rank += 1;

			return {
				id: `${summary.run_id}:${summary.target_key}`,
				summary,
				rank: ranked ? rank : null,
				// Identity, not baseline-hood: every drizzle-rs row is "us", including the second and
				// third API variants that are not the row the delta is measured against.
				isOurs: isDrizzleRsTarget(summary),
				barPct: this.#barPct(summary, view, { peak, peakLatency, peakCapacity }),
				barKind: this.sort === 'capacity' && view.state === 'lower-bound' ? 'bound' : 'measured',
				capacity: view,
				...this.#delta(summary, baselines.get(targetFamily(summary)) ?? null),
			};
		});
	}

	/**
	 * How the table is ordered, per sort mode.
	 *
	 * Capacity is the one that needs its own rule. A row with no measured peak never outranks a row
	 * that has one, whatever number it happens to carry: a lower bound of 40k is evidence the ramp
	 * stopped early, not evidence of beating a measured 12k, and letting the two sort together
	 * would turn "we did not find out" into a placement. Errored rows still sink to the bottom in
	 * every mode, because their numbers are not comparable at all.
	 */
	get #comparator(): (a: SummaryResult, b: SummaryResult) => number {
		if (this.sort === 'capacity') {
			return (a, b) =>
				this.#byErrorsThen(a, b, compareCapacity(this.capacity(a), this.capacity(b)));
		}
		if (this.sort === 'latency') {
			return (a, b) => this.#byErrorsThen(a, b, a.primary.latency.p95 - b.primary.latency.p95);
		}
		return (a, b) => this.#byErrorsThen(a, b, b.primary.rps.avg - a.primary.rps.avg);
	}

	#barPct(
		summary: SummaryResult,
		view: CapacityView,
		scale: { peak: number; peakLatency: number; peakCapacity: number },
	): string | null {
		if (this.sort === 'capacity') {
			// No number, no bar. An empty track is the honest drawing of "not measured"; a zero-width
			// bar reads as a measured zero and a full-width one would be a fabrication.
			if (!view.figure) return null;
			return `${Math.max(1.5, (view.tierValue / scale.peakCapacity) * 100).toFixed(1)}%`;
		}
		const fraction =
			this.sort === 'latency'
				? summary.primary.latency.p95 / scale.peakLatency
				: summary.primary.rps.avg / scale.peak;
		return `${Math.max(1.5, fraction * 100).toFixed(1)}%`;
	}

	/** Peak throughput for one row, in whichever of its four states it is. */
	capacity(summary: SummaryResult): CapacityView {
		return capacity(summary);
	}

	/**
	 * The objective every peak figure in this table was measured against, e.g. "at p99 < 25 ms".
	 *
	 * It is one value for the whole set, so it belongs in the column header rather than on each of
	 * the rows underneath it. Null when no row in view carries a figure, in which case the header
	 * says nothing rather than naming an objective nothing was measured against.
	 */
	get capacityObjective(): string | null {
		for (const summary of this.#scopedResults) {
			const figure = this.capacity(summary).figure;
			if (figure) return figure.qualifier;
		}
		return null;
	}

	/**
	 * Whether this set measured capacity at all.
	 *
	 * Computed over the OS scope, not over each `?db=` slice: switching database pills must never
	 * make the peak-throughput column appear and disappear, but switching operating system genuinely
	 * can, because whether a platform was measured for capacity is a fact about that platform's job.
	 * When it is false the column is left off and a note says why — twenty rows all reading "not
	 * measured" is technically honest and practically unreadable.
	 */
	get hasCapacity(): boolean {
		return anyMeasured(this.#scopedResults);
	}

	/** Capacity is the primary number wherever it exists, and nothing pretends it does when it does not. */
	get defaultSort(): RankingSort {
		return this.hasCapacity ? 'capacity' : 'throughput';
	}

	/**
	 * The orders this set can genuinely be put in. A cohort with no saturation measurement cannot be
	 * ordered by peak throughput, so `?sort=capacity` resolves to its default rather than producing
	 * a table of unranked rows in an order that means nothing.
	 */
	get availableSorts(): RankingSort[] {
		return this.hasCapacity ? ['capacity', 'throughput', 'latency'] : ['throughput', 'latency'];
	}

	/**
	 * The harness each comparison group on screen ran under, merged across the set's shards.
	 *
	 * Grouped by `fair.family`, not by database. One database can hold two groups — `sqlite` for the
	 * Rust stack and `sqlite-ts` for the Bun/TypeScript one — because a single-threaded runtime
	 * cannot be given a pool of 8 without the number being fiction. They get a line each, since two
	 * genuinely different harnesses cannot be described by one.
	 */
	get harnessRows(): HarnessRow[] {
		const present = [...new Set(this.#scopedResults.map((summary) => targetFamily(summary)))];
		return harnessRows(present, this.latest?.harness, familyLabel);
	}

	/**
	 * The harness governing one row, printed inside that row's expanded detail.
	 *
	 * The same fact appears twice on purpose. On the strip it is a per-database legend, which is
	 * where a reader checks that a family is internally consistent; on the row it is the scope of
	 * that row's own "vs drizzle-rs" delta, which is where a reader is most likely to mistake a
	 * stack difference for a library one.
	 */
	harnessFor(summary: SummaryResult): HarnessRow | null {
		const family = targetFamily(summary);
		return this.harnessRows.find((row) => row.family === family) ?? null;
	}

	/** True when the current `?db=` filter matched nothing at all. */
	get hasRankingRows(): boolean {
		return this.rankingRows.length > 0;
	}

	/**
	 * One tile per database: where drizzle-rs placed on it, and by how much against the best
	 * alternative *on that database*.
	 *
	 * These sit above the table and are deliberately not derived from it — the table is one global
	 * order and these are per-database standings, which is exactly the orientation the flat table
	 * does not give you. Each tile links to the table filtered to its database, so it doubles as
	 * the way in. Tiles are computed from the whole OS scope rather than from the current `?db=`
	 * filter, so they stay a complete index however the table is filtered — but they never reach
	 * across operating systems, because "1st of 4" would otherwise be a standing against a field
	 * measured on another machine.
	 *
	 * In-process-cache targets are left out of each database's field: they answer without touching
	 * the database, so placing drizzle-rs against one would not be a standing among comparable
	 * libraries.
	 */
	get verdicts(): DbVerdict[] {
		const byDb = new Map<DbProfile, SummaryResult[]>();
		for (const summary of this.#scopedResults) {
			if (isInProcessCache(summary.target_meta)) continue;
			const profile = dbProfile(summary);
			const bucket = byDb.get(profile);
			if (bucket) bucket.push(summary);
			else byDb.set(profile, [summary]);
		}

		const out: DbVerdict[] = [];
		for (const profile of DB_PROFILE_ORDER) {
			const bucket = byDb.get(profile);
			if (!bucket || bucket.length === 0) continue;

			const rows = [...bucket].sort(compareLeaderboard);
			const ourIndex = rows.findIndex((summary) => isDrizzleRsTarget(summary));
			if (ourIndex === -1) continue;

			const ours = rows[ourIndex];
			const label = dbLabel(profile);
			const standing = `${ordinal(ourIndex + 1)} of ${rows.length}`;
			const common = {
				db: profile,
				label,
				href: this.rankingUrl(profile, this.sort),
				active: this.db === profile,
				standing,
			};

			const best = rows.find((summary) => !isDrizzleRsTarget(summary));
			if (!best) {
				out.push({
					...common,
					margin: null,
					leads: true,
					detail: `drizzle-rs is the only library measured on ${label} in this set.`,
				});
				continue;
			}

			const bestName = targetDisplay(best).name;
			const delta = rowDelta(ours.primary.rps.avg, best.primary.rps.avg, true);
			out.push({
				...common,
				margin:
					delta === null
						? null
						: `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}% vs ${bestName}`,
				leads: ourIndex === 0,
				detail:
					delta === null
						? `Not comparable against ${bestName}.`
						: `${deltaSentence(delta, 'drizzle-rs', bestName, { better: 'faster', worse: 'slower' })} — the fastest other library measured on ${label} in this set. Opens the ranking filtered to ${label}.`,
			});
		}
		return out;
	}

	/** Errored targets sort last whichever column is chosen; their numbers are not comparable. */
	#byErrorsThen(a: SummaryResult, b: SummaryResult, tiebreak: number): number {
		const aBad = hasMaterialErrors(a);
		const bBad = hasMaterialErrors(b);
		if (aBad !== bBad) return aBad ? 1 : -1;
		return tiebreak;
	}

	/**
	 * The drizzle row each database's rows are measured against.
	 *
	 * Computed from the *whole* set, not the filtered view, so switching the `?db=` pills never
	 * changes a single delta — the number on a row means the same thing whichever way the table is
	 * currently sliced. Sorting first is what makes `pickBaseline` return the strongest drizzle
	 * result in each group rather than whichever one the artifact happened to list first.
	 *
	 * Keyed on the comparison group, not the database: a TypeScript SQLite row is measured against
	 * the drizzle target on its own runtime, because drizzle-rs-on-Rust and drizzle-orm-on-Bun
	 * differ by language and concurrency model before they differ by library.
	 */
	get #baselines(): Map<string, SummaryResult> {
		return baselinesByFamily([...this.#scopedResults].sort(compareLeaderboard));
	}

	/**
	 * Which databases this set actually produced, in the canonical order.
	 *
	 * These narrow the one table; they never change how a row in it is computed. Rank, bar and
	 * delta are the same numbers with "All" selected as with one database selected — the pills are
	 * a lens, not a different ranking.
	 */
	/**
	 * The operating-system scopes, as pills.
	 *
	 * Rendered even when the set has only one, so the reader always knows which platform they are
	 * looking at. A single unlabelled scope is the state that produced the original confusion.
	 */
	get osFilters(): FilterOption[] {
		return this.osScopes.map((scope) => ({
			label: `${scope.label} (${scope.count})`,
			title: scope.detail,
			href: this.rankingUrl(this.db, this.sort, scope.os),
			active: this.os === scope.os,
		}));
	}

	get dbFilters(): FilterOption[] {
		const present = new Set(this.#scopedResults.map((summary) => dbProfile(summary)));
		const families = DB_PROFILE_ORDER.filter((profile) => present.has(profile));
		return [
			{
				label: 'All',
				href: this.rankingUrl(null, this.sort),
				active: !this.db,
				title: 'Every database in this set, in one ranked table',
			},
			...families.map((profile) => ({
				label: dbLabel(profile),
				href: this.rankingUrl(profile, this.sort),
				active: this.db === profile,
				title: dbProfileDetail(profile),
			})),
		];
	}

	/**
	 * The sort pills, named in the same words as the columns they order.
	 *
	 * "peak throughput" only appears when this set has one, because a sort mode is a promise that
	 * the table can be put in that order — offering it over a table of "not measured" would be a
	 * control that does nothing.
	 */
	get sortOptions(): FilterOption[] {
		return this.availableSorts.map((sort) => ({
			label: SORT_LABELS[sort].label,
			title: SORT_LABELS[sort].title,
			href: this.rankingUrl(this.db, sort),
			active: this.sort === sort,
		}));
	}

	rankingUrl(db: string | null, sort: RankingSort, os: string | null = this.os): string {
		const params = new URLSearchParams();
		if (this.suite) params.set('suite', this.suite);
		if (this.status) params.set('status', this.status);
		if (db) params.set('db', db);
		// Omitted only when it is already the effective default, so the shortest URL and the
		// rendered order always agree.
		if (os && os !== this.defaultOs) params.set('os', os);
		if (sort !== this.defaultSort) params.set('sort', sort);
		const query = params.toString();
		return this.#basePath + (query ? '?' + query : '');
	}

	/** Short database name for the ranking's own column; the long form is the filter's tooltip. */
	dbName(summary: SummaryResult): string {
		return dbLabel(dbProfile(summary));
	}

	/** Full description of the row's database, for the database cell's tooltip. */
	dbDetail(summary: SummaryResult): string {
		return dbProfileDetail(dbProfile(summary));
	}

	isCache(summary: SummaryResult): boolean {
		return isInProcessCache(summary.target_meta);
	}

	/**
	 * How this row compares to the drizzle target *in this row's own comparison group*.
	 *
	 * One table holds every database, so the reference has to be named rather than assumed: a
	 * PostgreSQL row measured against a SQLite drizzle number would be comparing two engines and
	 * calling it a library comparison. The scope is the comparison group rather than the database,
	 * which is a finer cut wherever one database holds two — a Bun SQLite row is measured against
	 * drizzle-orm on Bun, not against drizzle-rs on Rust, because those differ by runtime and
	 * concurrency model before they differ by library. Where the set contains no drizzle row in a
	 * group, the cell says so instead of quietly borrowing the nearest one.
	 */
	#delta(
		summary: SummaryResult,
		baseline: SummaryResult | null,
	): {
		deltaText: string;
		deltaDirection: DeltaDirection;
		deltaTitle: string;
		deltaLabel: string;
	} {
		const group = familyLabel(targetFamily(summary));
		if (baseline === null) {
			return {
				deltaText: '—',
				deltaDirection: 'flat',
				deltaLabel: `vs drizzle on ${group}`,
				deltaTitle: `no drizzle row in the ${group} group in this set, so there is nothing directly comparable to measure this row against`,
			};
		}

		const reference = `${targetDisplay(baseline).name} on ${group}`;
		const deltaLabel = `vs ${targetDisplay(baseline).name} on ${group}`;
		if (summary === baseline) {
			return {
				deltaText: 'baseline',
				deltaDirection: 'flat',
				deltaLabel,
				deltaTitle: `the drizzle baseline row for ${group}; every other ${group} row is measured against this one, under the same harness`,
			};
		}
		if (isInProcessCache(summary.target_meta)) {
			return {
				deltaText: '—',
				deltaDirection: 'flat',
				deltaLabel,
				deltaTitle: `this target answers from an in-process cache and never crosses a database boundary, so it is not comparable to ${reference}`,
			};
		}
		if (hasMaterialErrors(summary)) {
			return {
				deltaText: 'errored',
				deltaDirection: 'flat',
				deltaLabel,
				deltaTitle: 'error rate above 0.5%: throughput is not comparable',
			};
		}

		// Throughput: higher is better, so the row's own sign is already the natural one.
		const delta = rowDelta(summary.primary.rps.avg, baseline.primary.rps.avg, true);
		if (delta === null) {
			return {
				deltaText: '—',
				deltaDirection: 'flat',
				deltaLabel,
				deltaTitle: `not comparable to ${reference}`,
			};
		}
		return {
			deltaText: `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`,
			deltaDirection: deltaDirection(delta),
			deltaLabel,
			deltaTitle: deltaSentence(delta, 'This library', reference, {
				better: 'faster',
				worse: 'slower',
			}),
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

	/**
	 * The single line of run-level metadata a list page is allowed: which commit, when, how many
	 * trials. Nothing else.
	 *
	 * This replaced a dump of counts — sets, results, target ids, runner class, cores — that was
	 * identical on every visit and could not change what anyone clicked. The counts that do matter
	 * are still on screen where they mean something: the number of rows is the list itself, and a
	 * job that fell short says so on its own row.
	 */
	get overviewMeta(): string {
		const latest = this.latest;
		if (!latest) return `${this.totalCohorts} job${this.totalCohorts === 1 ? '' : 's'}`;
		const trials = latest.manifest.trials.count;
		return `commit ${shortHash(latest.cohort.git)} · ${fmtDate(latest.cohort.start)} · ${trials} trial${trials === 1 ? '' : 's'} each`;
	}

	/** Same line for `/runs`, where the newest job is the one that dates the page. */
	get runsMeta(): string {
		const newest = this.cohorts[0];
		const count = this.totalCohorts;
		if (!newest) return `${count} job${count === 1 ? '' : 's'}`;
		return `${count} job${count === 1 ? '' : 's'} · commit ${shortHash(newest.git)} · ${fmtDate(newest.start)}`;
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
				detail: `busiest second ${fmtRps(p.rps.peak)}`,
				hint: "median requests/second across trials, at the paced suite's fixed offered load",
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
	 * Short forms for every SQL note on this page, computed together so two targets whose
	 * explanations open the same way are told apart rather than both collapsing to one label.
	 */
	#variantNotes = $derived(
		summarizeAll(
			this.results
				.map((summary) => targetDisplay(summary).sqlVariant)
				.filter((text): text is string => Boolean(text)),
		),
	);

	variantNote(summary: SummaryResult): QualitativeNote | null {
		const raw = targetDisplay(summary).sqlVariant;
		return raw ? (this.#variantNotes.get(raw.trim()) ?? null) : null;
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

	/**
	 * A filter with one option filters nothing. This site has published a single suite for its whole
	 * life, so the suite row was two pills — "all" and the only suite — taking a line on three pages
	 * to offer no choice. It appears the moment a second suite exists.
	 */
	get showSuiteFilter(): boolean {
		return this.suites.length > 1;
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
