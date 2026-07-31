import type { DeltaDirection } from './leaderboard';
import type { SummaryResult } from './types';

/**
 * The flat cross-family ranking's own vocabulary.
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
 * A database family as a band inside the single ranking table.
 *
 * Family separation used to be layout separation — a card per family, each with its own header
 * row. That made the page read as several small leaderboards and buried the one question a reader
 * arrives with. The information that made the separation honest is all still here; it just moved
 * into the grammar of one table: rank restarts at 01 inside a band, the bar scales within the band,
 * and the delta is measured against that band's own drizzle baseline. The divider is a pause on the
 * page background, not a box.
 */
export interface RankingFamily {
	key: string;
	/** Short label for the divider, e.g. "SQLite · embedded". */
	label: string;
	/** The full description sentence; the divider shows it as a tooltip, never inline. */
	note: string | null;
	/** Which machine the band ran on, right-aligned on the divider. */
	provenance: string;
	/** Anchor target so a verdict tile can link to its band without JavaScript. */
	anchor: string;
	/** False for the in-process-cache band, which is listed but not ranked. */
	ranked: boolean;
	rows: RankingRow[];
}

/**
 * "How does drizzle-rs stack up here", for one family.
 *
 * The comparison is against the strongest *other* library in the same family rather than a fixed
 * raw-driver baseline: the raw driver is not always present, and when it is it is not always the
 * one to beat. Naming the target in `versus` keeps the claim checkable.
 */
export interface FamilyVerdict {
	family: string;
	anchor: string;
	/** "1st of 4". */
	standing: string;
	/** "+3.1% vs rusqlite", or null when there is nothing to compare against. */
	margin: string | null;
	/** Whether drizzle-rs leads the family; drives the quiet accent, never the only signal. */
	leads: boolean;
	/** Long form for the tile's tooltip. */
	detail: string;
}

const ORDINALS = ['1st', '2nd', '3rd', '4th', '5th', '6th', '7th', '8th', '9th', '10th'];

export function ordinal(n: number): string {
	return ORDINALS[n - 1] ?? `${n}th`;
}
