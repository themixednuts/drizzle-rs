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
