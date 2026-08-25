import { fmtLatency, fmtPct, fmtRps } from './format';
import type { LatencyUnderLoad, Summary, SustainedStep } from './types';

/**
 * The one place that decides what a target's latency figure is.
 *
 * Two numbers on this site are latency and they mean different things:
 *
 *   - **latency at sustained load** — read at the highest rung of the ramp where the target still
 *     served the load it was offered. This measures the target.
 *   - **whole-ramp latency** (`primary.latency`) — every counted hold plateau's raw samples merged
 *     into one percentile. Past a target's throughput ceiling each further second of the ramp adds
 *     `VUs / throughput` of queueing, so this ranks targets by how far the ramp overshot them. It is
 *     kept because it is the upstream drizzle-benchmarks method, and it is the number that makes
 *     this suite's throughput figure comparable with theirs.
 *
 * They are never averaged, never compared, and never rendered with the same words — the same rule
 * `#lib/saturation` enforces for the two throughputs, for the same reason.
 *
 * Most published runs predate the sustained-load reading entirely, so `basis()` decides which of the
 * two a whole table is allowed to show, and it only says `sustained` when *every* row in scope
 * carries one. A column holding a mix of the two would be two measurements under one heading.
 */

export type LatencyBasis = 'sustained' | 'whole-ramp';

export interface LatencyView {
	/** The figure, formatted. Never null: falls back to the whole-ramp number, labelled as such. */
	text: string;
	/** Which of the two measurements `text` is. */
	basis: LatencyBasis;
	/** The p95 in milliseconds, for sorting and for the rail. */
	value: number;
	/** Short words that qualify the figure, e.g. "at 400 VUs" or "whole ramp". */
	note: string;
	/** The full explanation, for a tooltip. */
	detail: string;
	/** The rung the reading came from; null on the whole-ramp basis. */
	at: SustainedStep | null;
}

/**
 * The sustained-load block, or `null` when this run did not record one.
 *
 * Mirrors `readSaturation`: a block without a recognised `outcome` is treated as absent rather than
 * guessed at, so an artifact written by a future runner cannot be rendered as something it is not.
 */
export function readLatency(summary: Pick<Summary, 'latency'>): LatencyUnderLoad | null {
	const raw = summary.latency;
	if (!raw || typeof raw !== 'object' || !('outcome' in raw)) return null;

	switch (raw.outcome) {
		case 'measured':
			// Without a reading the block is malformed, and gets treated as absent.
			return raw.reference ? raw : null;
		case 'floor_above_knee':
		case 'floor_disqualified':
			return raw;
		default:
			return null;
	}
}

/**
 * Whether a whole table may show the sustained-load figure.
 *
 * `sustained` only when every row in scope has a reading — including the two `floor_*` outcomes,
 * which have a block but no figure, and so cannot fill the column either. Anything short of that
 * and the table shows the whole-ramp number for all rows and says so once in the heading, rather
 * than printing two different measurements down one column.
 */
export function latencyBasis(summaries: readonly Pick<Summary, 'latency'>[]): LatencyBasis {
	if (summaries.length === 0) return 'whole-ramp';
	return summaries.every((summary) => readLatency(summary)?.reference) ? 'sustained' : 'whole-ramp';
}

/**
 * The offered load every row is read at, when they agree on one.
 *
 * They normally do — a set runs one workload, so one ladder, so one reference rung — and when they
 * do the qualifier belongs in the column heading rather than repeated down every row. `null` when a
 * table spans runs whose ladders differ, and then each row has to carry its own.
 */
export function sharedReferenceLoad(summaries: readonly Pick<Summary, 'latency'>[]): number | null {
	const loads = new Set<number>();
	for (const summary of summaries) {
		const at = readLatency(summary)?.reference;
		if (!at) return null;
		loads.add(at.concurrency);
	}
	return loads.size === 1 ? [...loads][0] : null;
}

const WHOLE_RAMP_DETAIL =
	'Merged across every hold plateau of the ramp, up to 3000 concurrent. Past a target’s throughput ceiling each further second of the ramp contributes queueing rather than work, so this figure says how far the ramp overshot the target as much as how fast it answers. It is the upstream drizzle-benchmarks method, kept so the throughput figure beside it stays comparable with theirs.';

/** How a rung's retention reads in words, since the number alone does not say what it is. */
function retentionNote(step: SustainedStep): string {
	const served = fmtPct(Math.min(1, step.retention));
	return `served ${served} of the ${fmtRps(step.offered_rps)} req/s it was offered there`;
}

/**
 * One target's latency, on whichever basis the table is using.
 *
 * `basis` is passed in rather than decided per row: a row that happens to carry a reading still
 * shows the whole-ramp figure when the table it sits in cannot show that basis for every row.
 */
export function latencyView(
	summary: Pick<Summary, 'latency' | 'primary'>,
	basis: LatencyBasis,
): LatencyView {
	const wholeRamp: LatencyView = {
		text: fmtLatency(summary.primary.latency.p95),
		basis: 'whole-ramp',
		value: summary.primary.latency.p95,
		note: 'whole ramp',
		detail: WHOLE_RAMP_DETAIL,
		at: null,
	};

	if (basis === 'whole-ramp') return wholeRamp;

	const doc = readLatency(summary);
	const at = doc?.reference;
	// `latencyBasis` guarantees this, but a caller can pass a basis the row cannot honour and the
	// honest answer is the number this row actually has, not a blank.
	if (!doc || !at) return wholeRamp;

	// How far up the ramp this target got. Not where the figure was read — that is fixed — but it is
	// the thing a reader will want next, and it is already in the curve.
	const top = [...doc.curve].reverse().find((step) => step.sustained);

	return {
		text: fmtLatency(at.latency.p95),
		basis: 'sustained',
		value: at.latency.p95,
		note: `at ${at.concurrency} VUs`,
		detail:
			`p95 ${fmtLatency(at.latency.p95)} at ${at.concurrency} concurrent — the same offered load every row on this table is read at, so the column is one comparison rather than each target measured wherever it happened to give out. It ${retentionNote(at)}.` +
			(top && top.concurrency > at.concurrency
				? ` This target went on holding the load up to ${top.concurrency} concurrent.`
				: ''),
		at,
	};
}
