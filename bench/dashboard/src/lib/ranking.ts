import type { DeltaDirection } from './leaderboard';
import type { SummaryResult } from './types';

/**
 * The single cross-database ranking's own vocabulary.
 *
 * It lives in `lib` rather than beside the route because the row component renders it; a type that
 * both a route and a shared component depend on cannot sit inside the route without the component
 * having to reach back up into `routes/`.
 */

/** How the ranking is ordered. Both are URL state, so a sorted view is shareable. */
export type RankingSort = 'throughput' | 'latency';

export function parseRankingSort(value: string | null): RankingSort {
	return value === 'latency' ? 'latency' : 'throughput';
}

/**
 * One row of the one table.
 *
 * There is no band, no per-family rank restart and no per-family bar scale: rank is `01..N` across
 * every database in the set and the bar is scaled against the fastest row currently on screen. That
 * is the comparison the table is for — SQLite against PostgreSQL against Turso against SpacetimeDB
 * is the point, not an accident to be designed around.
 *
 * What the bands used to carry did not disappear, it moved onto the row: the database has its own
 * column (with the family's description on its tooltip), the machine has its own badge, and the
 * "vs drizzle-rs" delta is still measured against the drizzle row on *this* row's database.
 */
export interface RankingRow {
	id: string;
	summary: SummaryResult;
	/** 1-based position in the current view; the whole table is ranked, so never null. */
	rank: number;
	isOurs: boolean;
	/** Bar width as a percentage string, scaled within the rows currently on screen. */
	barPct: string;
	deltaText: string;
	deltaDirection: DeltaDirection;
	deltaTitle: string;
}

/**
 * "How does drizzle-rs place on this database", for one database.
 *
 * The tiles are orientation above a table that is deliberately not grouped: the table answers
 * "who is fastest here", and these answer "how does drizzle do against its own field", which is a
 * different question and the one most readers actually arrive with. Each links to the same table
 * filtered to that database, so a tile is also a way in.
 *
 * The comparison is against the strongest *other* library on the same database rather than a fixed
 * raw-driver baseline: the raw driver is not always present, and when it is it is not always the
 * one to beat. Naming the target in `margin` keeps the claim checkable.
 */
export interface DbVerdict {
	/** `DbProfile` value, used as the key and as the `?db=` parameter. */
	db: string;
	/** Short database name, e.g. "SQLite". */
	label: string;
	/** Link to the ranking filtered to this database. */
	href: string;
	/** True when the ranking is currently filtered to this database. */
	active: boolean;
	/** "1st of 4". */
	standing: string;
	/** "+3.1% vs rusqlite", or null when there is nothing to compare against. */
	margin: string | null;
	/** Whether drizzle-rs leads this database; drives the quiet accent, never the only signal. */
	leads: boolean;
	/** Long form for the tile's tooltip. */
	detail: string;
}

const ORDINALS = ['1st', '2nd', '3rd', '4th', '5th', '6th', '7th', '8th', '9th', '10th'];

export function ordinal(n: number): string {
	return ORDINALS[n - 1] ?? `${n}th`;
}
