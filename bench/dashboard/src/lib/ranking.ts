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
 * There is no band and no per-family rank restart: rank is `01..N` across every database in the set.
 * That is the comparison the table is for — SQLite against PostgreSQL against Turso against
 * SpacetimeDB is the point, not an accident to be designed around.
 *
 * Every row's mark sits on one logarithmic rail (see `lib/rail.ts`), which is what makes the flat
 * table readable. The field spans more than a decade in both throughput and latency, and the fastest
 * row serves from an in-process cache; on a linear scale that row took the full width and pushed
 * everything doing real work into a stub against the left edge.
 *
 * What the bands used to carry did not disappear, it moved onto the row: the database sits on the
 * target's own note line, and the machine is stated once for the whole table.
 *
 * Distances are measured to positions in the field — the top of the table, and the row directly
 * ahead — rather than to a nominated target. A table that measures everything against its author's
 * row is answering a different question than the one it appears to be asking, and readers correctly
 * discount it. Where a row sits, and what it gave up to sit there, is the whole finding.
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
	 * Where this row's mark sits on the ratio rail, as a CSS percentage. `null` when the sorted
	 * column has no number for this row, so the track renders empty rather than putting a dot at
	 * the origin — on a log axis there is no origin to put it at.
	 */
	railLeft: string | null;
	/** `bound` draws the mark as an open-ended floor rather than a measurement. */
	barKind: 'measured' | 'bound';
	/** Peak throughput at the objective, in whichever of its four states this row is in. */
	capacity: CapacityView;
	/**
	 * Distance to the top of the table on the column it is sorted by, e.g. "−38.2%". Empty on the
	 * leading row, and on any row the sorted column has no comparable number for.
	 */
	gapText: string;
	gapTitle: string;
	/**
	 * Distance to the row directly above. This is where a field's clusters show: four rows within a
	 * percent of each other are one result, however far apart their gaps to the top are.
	 */
	intervalText: string;
	intervalTitle: string;
	/** The saturation ramp as a row-height sparkline; `null` when this row has no ramp. */
	ramp: RampSpark | null;
}

/** One row's ramp, reduced to what a sparkline draws. */
export interface RampSpark {
	/** Request rate at each concurrency step, in ramp order. */
	values: number[];
	/** Which step the peak was taken at, or `null` when no peak was found. */
	peakIndex: number | null;
	/** Spoken description, since the shape carries meaning a screen reader cannot see. */
	label: string;
}

/**
 * A signed percentage difference, in the direction the sorted column reads.
 *
 * Higher-is-better columns print a negative number for "behind"; lower-is-better ones print a
 * positive number for the same thing, because on a latency column being behind means a bigger
 * figure. Both are the natural sign of `value − reference`, so nothing is flipped on the reader's
 * behalf and every number on the column can be checked against the two it was computed from.
 *
 * A difference that rounds to nothing is a dead heat rather than a measurement: "−0.0%" reads as a
 * rounding artefact, so it is printed as "=".
 *
 * Past ten times the reference the percentage stops being readable and stops fitting: ordering the
 * table by latency puts a 1.8 ms leader against a 5.9 s tail, which is "+327,000%" — a figure no
 * reader parses and no column holds. Beyond that point it is printed as a multiplier. The leading
 * plus remains so the column does not switch from a signed difference to an unsigned ratio.
 */
export function gapPercent(value: number, reference: number): string | null {
	if (!Number.isFinite(value) || !Number.isFinite(reference) || reference <= 0) return null;

	const ratio = value / reference;
	if (ratio >= 10) return `+${ratio < 100 ? ratio.toFixed(1) : ratio.toFixed(0)}×`;

	const pct = (ratio - 1) * 100;
	if (Math.abs(pct) < 0.05) return '=';
	return `${pct > 0 ? '+' : '−'}${Math.abs(pct).toFixed(1)}%`;
}
