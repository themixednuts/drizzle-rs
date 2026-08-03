import type { DeltaDirection } from './leaderboard';
import type { CapacityView } from './saturation';
import type { SummaryResult } from './types';

/**
 * The single cross-database ranking's own vocabulary.
 *
 * It lives in `lib` rather than beside the route because the row component renders it; a type that
 * both a route and a shared component depend on cannot sit inside the route without the component
 * having to reach back up into `routes/`.
 */

/**
 * How the ranking is ordered. All three are URL state, so a sorted view is shareable.
 *
 * `capacity` is the saturation suite's peak throughput; `throughput` is the paced suite's request
 * rate at a fixed offered load. They are two different measurements and they are deliberately two
 * different sort modes rather than one column that changes meaning — the sort pills name which one
 * is active in the same words the columns use.
 */
export type RankingSort = 'capacity' | 'throughput' | 'latency';

const SORTS: RankingSort[] = ['capacity', 'throughput', 'latency'];

/**
 * Which sort a URL asks for, narrowed to the ones this set can actually be put in.
 *
 * `available` is the set's own list: a cohort that measured no capacity cannot be ordered by peak
 * throughput, and honouring `?sort=capacity` there would produce a table in which every row is
 * unranked and the order between them means nothing — a control that appears to have worked while
 * doing nothing. It resolves to the set's default instead, which is what the active pill shows.
 *
 * `fallback` is that default: capacity wherever the set has it, since capacity is the primary
 * number, and the paced rate otherwise. Both are visible in the pills, so neither is a hidden
 * choice.
 */
export function parseRankingSort(
	value: string | null,
	fallback: RankingSort,
	available: readonly RankingSort[] = SORTS,
): RankingSort {
	return available.find((sort) => sort === value) ?? fallback;
}

/** What each sort mode is called, and what the words mean. Used by the pills and the columns. */
export const SORT_LABELS: Record<RankingSort, { label: string; title: string }> = {
	capacity: {
		label: 'peak throughput',
		title:
			'Highest sustained request rate that still met the latency objective, from the unpaced saturation ramp. Rows without a measured peak sort below every row that has one.',
	},
	throughput: {
		label: 'throughput at fixed load',
		title:
			'Request rate under the paced suite, where the generator offers a fixed load. This is a latency-at-a-known-rate reading, not a capacity figure.',
	},
	latency: {
		label: 'latency',
		title: 'p95 response latency under the paced suite, lowest first.',
	},
};

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
	/**
	 * 1-based position in the current view.
	 *
	 * `null` when the sorted column has no comparable number for this row — which only happens
	 * under `sort=capacity`, where a row whose peak was never measured, or was only bounded from
	 * below, gets no rank at all. Position is the thing readers trust most on a ranked table, so a
	 * row is only given one when the data supports it; the others print a dash and say why in
	 * their own cell.
	 */
	rank: number | null;
	isOurs: boolean;
	/**
	 * Bar width as a percentage string, scaled within the rows currently on screen. `null` when the
	 * sorted column has no number for this row, so the track renders empty rather than at zero.
	 */
	barPct: string | null;
	/** `bound` draws the bar as an open-ended floor rather than a measurement. */
	barKind: 'measured' | 'bound';
	/** Peak throughput at the objective, in whichever of its four states this row is in. */
	capacity: CapacityView;
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
