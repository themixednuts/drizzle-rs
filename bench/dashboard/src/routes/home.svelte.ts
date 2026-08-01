import { page } from '$app/state';
import { boxWhiskerExtent, rpsBox } from '#lib/boxplot';
import { fmtCpu, fmtDate, fmtLatency, fmtPct, fmtRps, shortHash, suiteLabel } from '#lib/format';
import {
	deltaDirection,
	deltaSentence,
	groupTargets,
	rowDelta,
	type DeltaDirection,
	type TargetGroup,
} from '#lib/leaderboard';
import {
	dbProfile,
	dbProfileLabel,
	isDrizzleRsTarget,
	isInProcessCache,
	targetDisplay,
	type DbProfile,
} from '#lib/target-display';
import { cohortSearchText } from '#lib/cohort-search';
import { summarizeAll, type QualitativeNote } from '#lib/qualitative';
import type { FilterOption } from '#lib/components/FilterPills.svelte';
import {
	ordinal,
	parseRankingSort,
	type FamilyVerdict,
	type RankingFamily,
	type RankingRow,
	type RankingSort,
} from '#lib/ranking';
import type { Manifest, RunCohort, RunIndexEntry, SummaryResult } from '#lib/types';

interface RunsPageData {
	runs: RunIndexEntry[];
	q?: string;
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

/** Short names for the ranking's database column — the long forms are in `dbProfileLabel`. */
const DB_NAMES: Record<DbProfile, string> = {
	sqlite: 'SQLite',
	turso: 'Turso',
	postgres: 'PostgreSQL',
	spacetimedb: 'SpacetimeDB',
	other: 'other',
};

function dbLabel(profile: DbProfile): string {
	return DB_NAMES[profile];
}

/** What kind of thing the database is, for the divider's short label. */
const FAMILY_QUALIFIER: Partial<Record<DbProfile, string>> = {
	sqlite: 'embedded',
	turso: 'embedded',
	postgres: 'over TCP',
};

function familyLabel(key: string, fallback: string): string {
	const name = DB_NAMES[key as DbProfile];
	if (!name) return fallback;
	const qualifier = FAMILY_QUALIFIER[key as DbProfile];
	return qualifier ? `${name} · ${qualifier}` : name;
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
	/** Ranking view state, both in the URL so a filtered ranking is a shareable address. */
	db = $derived(page.url.searchParams.get('db'));
	sort = $derived(parseRankingSort(page.url.searchParams.get('sort')));

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

	/**
	 * Rows are grouped by database family instead of being poured into one RPS-sorted table:
	 * an embedded SQLite file, a TCP PostgreSQL connection and an in-process cache are not
	 * doing the same work, so a single ranking would imply a comparison that does not hold.
	 */
	sections: LeaderboardSection[] = $derived(
		groupTargets(this.results, compareLeaderboard).map((group) => this.#toSection(group)),
	);

	/**
	 * The flat ranking: every target in the set, in one order, across database families.
	 *
	 * The sectioned-by-family leaderboard is gone from this page. Splitting the table was how the
	 * old design stopped a reader inferring "SQLite beats PostgreSQL" from adjacency, but it cost
	 * the page its headline — you could not see, at a glance, where drizzle-rs stood. The honesty
	 * now rides on three things that are always on screen instead of on the layout: the `database`
	 * column, the note under each name (an in-process cache says so in words), and the footnote
	 * under the table pointing at Repeatability and Method. `/compare` keeps the amber callout for
	 * the case where someone actually puts two incomparable jobs side by side.
	 */
	get rankingRows(): RankingRow[] {
		const rows = this.results.filter((summary) => !this.db || dbProfile(summary) === this.db);
		const ordered = [...rows].sort(
			this.sort === 'latency'
				? (a, b) => this.#byErrorsThen(a, b, a.primary.latency.p95 - b.primary.latency.p95)
				: (a, b) => this.#byErrorsThen(a, b, b.primary.rps.avg - a.primary.rps.avg),
		);

		// Scaled to the rows on screen, not to every row that exists. With "all databases" selected
		// the in-memory-cache target is several times faster than anything doing real per-request
		// work, and an absolute scale would squash every other bar into an unreadable stub. The
		// number beside each bar is always the real one, so the bar is a shape cue and never the
		// source of a value.
		const baselineRps = this.#baselineRps;
		const peak = Math.max(1, ...ordered.map((summary) => summary.primary.rps.avg));
		const peakLatency = Math.max(1, ...ordered.map((summary) => summary.primary.latency.p95));

		return ordered.map((summary, index) => {
			const isOurs = isDrizzleRsTarget(summary);
			const fraction =
				this.sort === 'latency'
					? summary.primary.latency.p95 / peakLatency
					: summary.primary.rps.avg / peak;
			return {
				id: `${summary.run_id}:${summary.target_key}`,
				summary,
				rank: index + 1,
				isOurs,
				barPct: `${Math.max(1.5, fraction * 100).toFixed(1)}%`,
				...this.#delta(summary, baselineRps, isOurs),
			};
		});
	}

	/**
	 * The ranking as one table of family bands.
	 *
	 * Each band is ranked, scaled and compared entirely within itself — that is what keeps a single
	 * flat table from implying that a SQLite number and a PostgreSQL number are the same
	 * measurement. Family separation stops being layout separation: no card per family, no repeated
	 * header row, just a divider that reads as a pause. The `?db=` pills choose which bands show,
	 * never how a band is computed.
	 */
	get rankingFamilies(): RankingFamily[] {
		return this.sections
			.filter((section) => !this.db || section.key === this.db)
			.map((section) => {
				const peak = Math.max(1, ...section.rows.map((row) => row.summary.primary.rps.avg));
				const label = familyLabel(section.key, section.label);
				return {
					key: section.key,
					label,
					// The long description moves to the divider's tooltip; it never sits inline.
					note: section.note ?? (label === section.label ? null : section.label),
					provenance: [...new Set(section.shards.map((shard) => shard.os))].join(' · '),
					anchor: `family-${section.key}`,
					ranked: section.ranked,
					rows: section.rows.map((row) => ({
						id: row.id,
						summary: row.summary,
						rank: row.rank ?? 0,
						isOurs: row.isBaseline,
						barPct: `${Math.max(1.5, (row.summary.primary.rps.avg / peak) * 100).toFixed(1)}%`,
						deltaText: row.deltaText,
						deltaDirection: row.deltaDirection,
						deltaTitle: row.deltaTitle,
					})),
				};
			});
	}

	/** True when the current `?db=` filter matched no family at all. */
	get hasFamilies(): boolean {
		return this.rankingFamilies.length > 0;
	}

	/**
	 * One verdict per family: where drizzle-rs placed, and by how much against the best alternative.
	 *
	 * The comparison is the strongest *other* library in the same family rather than a fixed
	 * raw-driver baseline — the raw driver is not always present, and when it is it is not always
	 * the one to beat. Naming the target keeps the claim checkable. Built from the rows the table
	 * renders, so a tile and its band can never disagree.
	 */
	get verdicts(): FamilyVerdict[] {
		const out: FamilyVerdict[] = [];
		for (const family of this.rankingFamilies) {
			if (!family.ranked) continue;
			const ours = family.rows.find((row) => isDrizzleRsTarget(row.summary));
			if (!ours) continue;

			const standing = `${ordinal(ours.rank)} of ${family.rows.length}`;
			const best = family.rows
				.filter((row) => !isDrizzleRsTarget(row.summary))
				.sort((a, b) => b.summary.primary.rps.avg - a.summary.primary.rps.avg)[0];

			if (!best) {
				out.push({
					family: family.label,
					anchor: family.anchor,
					standing,
					margin: null,
					leads: true,
					detail: `drizzle-rs is the only library measured on ${family.label}.`,
				});
				continue;
			}

			const bestName = targetDisplay(best.summary).name;
			const delta = rowDelta(ours.summary.primary.rps.avg, best.summary.primary.rps.avg, true);
			out.push({
				family: family.label,
				anchor: family.anchor,
				standing,
				margin:
					delta === null
						? null
						: `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}% vs ${bestName}`,
				leads: ours.rank === 1,
				detail:
					delta === null
						? `Not comparable against ${bestName}.`
						: `${deltaSentence(delta, 'drizzle-rs', bestName, { better: 'faster', worse: 'slower' })} — the fastest other library on ${family.label}.`,
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

	get #baselineRps(): number | null {
		const ours = this.results
			.filter((summary) => isDrizzleRsTarget(summary))
			.sort(compareLeaderboard)[0];
		return ours?.primary.rps.avg ?? null;
	}

	/** Which database families this set actually produced, in the canonical order. */
	get dbFilters(): FilterOption[] {
		const present = new Set(this.results.map((summary) => dbProfile(summary)));
		const families = (
			['sqlite', 'turso', 'postgres', 'spacetimedb', 'other'] as DbProfile[]
		).filter((profile) => present.has(profile));
		return [
			{ label: 'All', href: this.rankingUrl(null, this.sort), active: !this.db },
			...families.map((profile) => ({
				label: dbLabel(profile),
				href: this.rankingUrl(profile, this.sort),
				active: this.db === profile,
			})),
		];
	}

	get sortOptions(): FilterOption[] {
		return (['throughput', 'latency'] as RankingSort[]).map((sort) => ({
			label: sort,
			href: this.rankingUrl(this.db, sort),
			active: this.sort === sort,
		}));
	}

	rankingUrl(db: string | null, sort: RankingSort): string {
		const params = new URLSearchParams();
		if (this.suite) params.set('suite', this.suite);
		if (this.status) params.set('status', this.status);
		if (db) params.set('db', db);
		if (sort !== 'throughput') params.set('sort', sort);
		const query = params.toString();
		return this.#basePath + (query ? '?' + query : '');
	}

	/** Short database name for the ranking's own column; the long form is the filter's tooltip. */
	dbName(summary: SummaryResult): string {
		return dbLabel(dbProfile(summary));
	}

	dbDetail(summary: SummaryResult): string {
		return dbProfileLabel(dbProfile(summary));
	}

	isCache(summary: SummaryResult): boolean {
		return isInProcessCache(summary.target_meta);
	}

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

		// Throughput: higher is better, so the row's own sign is already the natural one.
		const delta = rowDelta(summary.primary.rps.avg, baselineRps, true);
		if (delta === null) {
			return { deltaText: '-', deltaDirection: 'flat', deltaTitle: 'not comparable' };
		}
		return {
			deltaText: `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`,
			deltaDirection: deltaDirection(delta),
			deltaTitle: deltaSentence(delta, 'This library', 'drizzle-rs', {
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
