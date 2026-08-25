import { page } from '$app/state';
import {
	boxWhiskerExtent,
	rpsBox,
	type BoxWhiskerDatum,
	type BoxWhiskerExtent,
} from '#lib/boxplot';
import { fmtDate, fmtLatency, fmtPct, fmtRps, shortHash, suiteLabel } from '#lib/format';
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
import {
	anyMeasured,
	buildCurve,
	capacity,
	compareCapacity,
	readSaturation,
	type CapacityView,
} from '#lib/saturation';
import { harnessRows, type HarnessRow } from '#lib/harness';
import { buildRail, railLeft, type Rail } from '#lib/rail';
import { buildScope, type ScopeView } from '#lib/scope';
import {
	latencyBasis,
	latencyView,
	sharedReferenceLoad,
	type LatencyBasis,
	type LatencyView,
} from '#lib/service-latency';
import type { ReplayView } from '#lib/replay';
import { osScopes, type OsScope } from '#lib/os';
import type { FilterOption } from '#lib/components/FilterPills.svelte';
import {
	gapPercent,
	parseRankingSort,
	SORT_LABELS,
	type RampSpark,
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
	/**
	 * A ramp for every target in the set, before the ranking's filters narrow it. Null when the
	 * artifacts carry no load levels to play against.
	 */
	replay?: ReplayView | null;
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

/** The row's identity, shared by the table and the plot so a hover crosses between them. */
function rowId(summary: SummaryResult): string {
	return `${summary.run_id}:${summary.target_key}`;
}

/** One measured distance between two rows: what to print, and what it means. */
interface Distance {
	text: string;
	title: string;
}

type ComparableMetric =
	| { kind: 'measured'; value: number }
	| { kind: 'errors'; rate: number }
	| { kind: 'unmeasured' };

/** `{text, title}` under a column's own names, so both distances can spread into the same row. */
function prefixed<K extends 'gap' | 'interval'>(
	key: K,
	distance: Distance,
): { [P in `${K}Text` | `${K}Title`]: string } {
	return {
		[`${key}Text`]: distance.text,
		[`${key}Title`]: distance.title,
	} as { [P in `${K}Text` | `${K}Title`]: string };
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
	/**
	 * The row under the pointer, whichever view the pointer is in.
	 *
	 * Held here rather than in either component because the plot and the table are two drawings of
	 * one set of rows: hovering a point has to lift a row twenty lines down, and hovering that row
	 * has to light its point back up.
	 */
	hoverRowId = $state<string | null>(null);
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
		const ordered = this.#orderedRows;
		const rail = this.rail;
		const metric = SORT_LABELS[this.sort].label;

		// Every distance on the table is measured on the column the table is sorted by, so the two
		// always agree. Rows the sorted column has no number for contribute nothing and receive
		// nothing: they are skipped as references and print no gap of their own.
		const values = ordered.map((summary) => this.#comparable(summary));
		const leadIndex = values.findIndex((value) => value.kind === 'measured');
		const lead = leadIndex === -1 ? null : ordered[leadIndex];
		const leadValue = leadIndex === -1 ? null : values[leadIndex];

		// Ranks are handed out only to rows the sorted column actually measured, and they run
		// 01..N over those. Under `sort=capacity` that means the unmeasured rows carry no number
		// at all rather than a number that would read as a placement.
		let rank = 0;

		return ordered.map((summary, index) => {
			const view = this.capacity(summary);
			const ranked = this.sort !== 'capacity' || view.rankable;
			if (ranked) rank += 1;

			// The nearest row above that the sorted column did measure. Skipping the unmeasured ones
			// keeps an interval a distance between two real figures rather than a hole in the column.
			let aheadIndex = index - 1;
			while (aheadIndex >= 0 && values[aheadIndex].kind !== 'measured') aheadIndex -= 1;
			const aheadValue = aheadIndex === -1 ? null : values[aheadIndex];

			return {
				id: rowId(summary),
				summary,
				rank: ranked ? rank : null,
				// Identity and nothing more: it tints the row so a reader can find it, and it changes no
				// number anywhere on the page.
				isOurs: isDrizzleRsTarget(summary),
				railLeft: railLeft(rail.at(this.#railValue(summary, view))),
				barKind: this.sort === 'capacity' && view.state === 'lower-bound' ? 'bound' : 'measured',
				capacity: view,
				ramp: this.#ramp(summary),
				...prefixed(
					'gap',
					this.#distance(
						values[index],
						index === leadIndex || leadValue?.kind !== 'measured' ? null : leadValue.value,
						lead && index !== leadIndex ? targetDisplay(lead).name : null,
						metric,
						'the row leading this order',
						true,
					),
				),
				...prefixed(
					'interval',
					this.#distance(
						values[index],
						aheadValue?.kind === 'measured' ? aheadValue.value : null,
						aheadIndex === -1 ? null : targetDisplay(ordered[aheadIndex]).name,
						metric,
						'the row directly above',
						false,
					),
				),
			};
		});
	}

	/**
	 * A distance from one row to another on the sorted column, as text plus its explanation.
	 *
	 * Written once and called twice because the gap and the interval differ only in which row they
	 * point at. `reference` being null covers both the row that *is* the reference and the case where
	 * there is nothing above it to measure against — neither is a failure, and both print a dash
	 * rather than a zero, which would claim a measured dead heat.
	 */
	#distance(
		value: ComparableMetric,
		reference: number | null,
		referenceName: string | null,
		metric: string,
		relation: string,
		toLeader: boolean,
	): Distance {
		if (value.kind === 'errors') {
			return {
				text: '—',
				title: `${metric} was excluded because ${fmtPct(value.rate)} of requests failed, above the 0.50% ranking limit.`,
			};
		}
		if (value.kind === 'unmeasured') {
			return {
				text: '—',
				title: `${metric} was not measured for this row, so it has no distance to ${relation}.`,
			};
		}
		if (reference === null || referenceName === null) {
			return {
				text: '—',
				title: toLeader
					? 'This row leads the order. Every gap in this column is measured to it.'
					: `Nothing above this row carries a ${metric} figure to measure against.`,
			};
		}

		const printed = gapPercent(value.value, reference);
		if (printed === null) return { text: '—', title: `Not comparable to ${referenceName}.` };
		if (printed === '=') return { text: '=', title: `Level with ${referenceName} on ${metric}.` };
		return {
			text: printed,
			title: `${printed} on ${metric} against ${referenceName}, ${relation}.`,
		};
	}

	/** This row's figure on the sorted column, or the reason it cannot be compared. */
	#comparable(summary: SummaryResult): ComparableMetric {
		if (hasMaterialErrors(summary)) return { kind: 'errors', rate: summary.primary.err };
		const value = this.#railValue(summary, this.capacity(summary));
		return Number.isFinite(value) && value > 0
			? { kind: 'measured', value }
			: { kind: 'unmeasured' };
	}

	/**
	 * The row's ramp, reduced to a sparkline.
	 *
	 * Read from the saturation artifact the row already carries, so drawing it costs no extra
	 * request; a row from a run that measured no ramp simply has none, and the cell stays empty
	 * rather than showing a flat line that would read as a measurement.
	 */
	#ramp(summary: SummaryResult): RampSpark | null {
		const doc = readSaturation(summary);
		if (!doc) return null;
		const curve = buildCurve(doc);
		if (curve.points.length < 2) return null;

		const peak = curve.peakIndex === null ? null : curve.points[curve.peakIndex];
		return {
			values: curve.points.map((point) => point.rps),
			peakIndex: curve.peakIndex,
			label: peak
				? `Ramp over ${curve.points.length} concurrency steps, peaking at ${fmtRps(peak.rps)} requests per second at ${peak.concurrency} concurrent.`
				: `Ramp over ${curve.points.length} concurrency steps, with no peak found.`,
		};
	}

	/** The rows currently in view, in the current order. */
	get #orderedRows(): SummaryResult[] {
		const rows = this.#scopedResults.filter(
			(summary) => !this.db || dbProfile(summary) === this.db,
		);
		return [...rows].sort(this.#comparator);
	}

	/**
	 * The one axis every row is drawn against.
	 *
	 * Built over the rows on screen, not over every row that exists: filtering to one database
	 * re-scales the rail to that database's field rather than leaving it stretched to fit rows that
	 * are no longer shown. The printed number beside each mark is always the real one.
	 */
	get rail(): Rail {
		const values = this.#orderedRows.map((summary) =>
			this.#railValue(summary, this.capacity(summary)),
		);
		return buildRail(values, this.sort === 'latency' ? fmtLatency : fmtRps);
	}

	/** Which number this row contributes to the rail, per sort mode. */
	#railValue(summary: SummaryResult, view: CapacityView): number {
		if (this.sort === 'capacity') return view.figure ? view.tierValue : Number.NaN;
		// The rail, the order and the printed figure are the same number by construction. Sorting on
		// the whole-ramp percentile while printing the sustained-load one would put the table in an
		// order its own column does not explain.
		if (this.sort === 'latency') return this.latency(summary).value;
		return summary.primary.rps.avg;
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
			return (a, b) => this.#byErrorsThen(a, b, this.latency(a).value - this.latency(b).value);
		}
		return (a, b) => this.#byErrorsThen(a, b, b.primary.rps.avg - a.primary.rps.avg);
	}

	/** Peak throughput for one row, in whichever of its four states it is. */
	capacity(summary: SummaryResult): CapacityView {
		return capacity(summary);
	}

	/**
	 * Which latency the table is showing: the one measured at sustained load, or the whole-ramp
	 * figure that includes the queue the ramp built.
	 *
	 * Decided over the operating-system scope rather than the `?db=` slice, for the same reason
	 * `hasCapacity` is: switching database pills must not change what a column means. It only says
	 * `sustained` when every row in scope carries a reading — a column holding a mix would be two
	 * measurements under one heading.
	 */
	get latencyBasis(): LatencyBasis {
		return latencyBasis(this.#scopedResults);
	}

	/**
	 * The load every row's latency is read at, when the table's rows agree on one.
	 *
	 * The reading rung is fixed by the ladder rather than chosen per target, so this is normally a
	 * single number for the whole table — which means it belongs in the heading, not repeated down
	 * twenty-seven rows.
	 */
	get latencyLoad(): number | null {
		return this.latencyBasis === 'sustained' ? sharedReferenceLoad(this.#scopedResults) : null;
	}

	/** One row's latency on the table's basis, with the words that say which measurement it is. */
	latency(summary: SummaryResult): LatencyView {
		return latencyView(summary, this.latencyBasis);
	}

	/**
	 * The field on two axes, drawn above the table.
	 *
	 * The page opens on this rather than on the filter bar, and rather than on any single target's
	 * ramp. A filter bar is chrome — it tells a reader what they may adjust before telling them
	 * anything worth adjusting — and one target's curve is an assertion about which target matters
	 * before the reader has seen the field.
	 *
	 * The plot is neither. It shows every row at once, positioned by the two quantities the table has
	 * to flatten into one order, and it is the one view on the site where the trade a target made is
	 * visible rather than inferred. It is also the way in: hovering a point lifts its row below.
	 *
	 * Built from the rows currently in view, so filtering the table re-frames the plot with it.
	 *
	 * `$derived` rather than a getter, unlike almost everything else on this class. A prop is read
	 * many times over the course of one render, and a getter would rebuild the whole point set on
	 * each read — which is not just wasted work: the plot compares points by object identity when it
	 * decides which ones to label, and two reads returning two sets of equal-but-distinct objects
	 * made every labelled point appear twice.
	 */
	scope: ScopeView = $derived(
		buildScope(this.#orderedRows.map((summary) => ({ id: rowId(summary), summary }))),
	);

	/**
	 * The ramp the plot above is a snapshot of, narrowed to the same rows the table is showing.
	 *
	 * The server loads a line for every target in the set; the filters decide which of them this is.
	 * That is what makes the picker beside the chart a control per driver in the current view rather
	 * than an arbitrary subset — switching to PostgreSQL leaves exactly the PostgreSQL drivers, and
	 * the chart and the table can never disagree about what is in scope.
	 *
	 * Ordered by rate, so the handful drawn before the reader touches anything are the fastest.
	 *
	 * `null` when nothing in view has a load level to play against, and the section is left out
	 * rather than drawing an empty playback.
	 */
	replay: ReplayView | null = $derived.by(() => {
		const loaded = this.#data().replay;
		if (!loaded) return null;

		const series = loaded.series
			.filter((entry) => (!this.os || entry.os === this.os) && (!this.db || entry.db === this.db))
			.sort((a, b) => (b.rps ?? 0) - (a.rps ?? 0));
		if (series.length === 0) return null;

		// Re-scaled to what is in view: filtering to one database should not leave the axis stretched
		// to a ramp that is no longer on screen.
		const maxVus = Math.max(...series.flatMap((entry) => entry.points.map((point) => point.vus)));
		return maxVus > 0 ? { series, maxVus } : null;
	});

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

	/** Errored targets sort last whichever column is chosen; their numbers are not comparable. */
	#byErrorsThen(a: SummaryResult, b: SummaryResult, tiebreak: number): number {
		const aBad = hasMaterialErrors(a);
		const bBad = hasMaterialErrors(b);
		if (aBad !== bBad) return aBad ? 1 : -1;
		return tiebreak;
	}

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

	/**
	 * The row's trial spread, scaled to itself.
	 *
	 * Every figure on the table is a median across five trials, and the five do not always agree.
	 * Where they disagree by more than the interval to the row above, the order between those two
	 * rows is not a result — and this is the only thing on the page that can say so.
	 *
	 * The extent comes from this row's own trials rather than from the table's, because the variation
	 * being drawn is a few percent while the field spans more than a decade: on a shared axis every
	 * box would be a pixel wide.
	 */
	spreadFigure(summary: SummaryResult): { box: BoxWhiskerDatum; extent: BoxWhiskerExtent } {
		const box = rpsBox(summary);
		return { box, extent: boxWhiskerExtent([box]) };
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
