/**
 * The ratio rail: one logarithmic axis shared by every row of the ranking.
 *
 * This suite reports 767 to 9,800 requests a second, and 1.8ms to 4.78s, in the same table. On a
 * linear scale that is unreadable in a specific and misleading way: the fastest row takes the full
 * width, the middle of the field collapses into a band a few pixels wide, and the bottom third is
 * indistinguishable from zero. Worse, the fastest row here serves from an in-process cache and does
 * no per-request database work — so a linear scale lets one row that is not doing the job set the
 * scale for every row that is.
 *
 * A log scale fixes the thing that actually matters for this data. Equal distance is equal ratio, so
 * "twice as fast" is the same gap everywhere on the axis, whether it happens at 800 or at 8,000.
 * That is the comparison a reader is making, and it is the one the axis should be able to answer by
 * eye.
 *
 * The mark is a dot, not a bar. A bar encodes a value by its length measured from zero, and log(0)
 * is negative infinity — there is no zero on this axis to measure from, so a bar would be drawing a
 * quantity the scale cannot express. A dot states a position, which is exactly what a log axis has.
 */

/**
 * Ladders of round numbers, coarsest first. Bounds and ticks both land on one of these, so an axis
 * never ends on an odd number.
 *
 * More than one, because a single ladder cannot serve both shapes of data this site plots. The
 * 1-2-5 rungs are the nicest to read and are right whenever the values span a decade or more — the
 * full run's request rates go 766 to 9,800, and snapping those to 500..10,000 spends 85% of the
 * axis on data. They are wrong when the values are close together: a preview run's rates sit
 * between 440 and 492, which the same rule snaps to 200..500 and draws as a single clump against
 * the right edge, using 12% of the width. `buildRail` walks these in order and takes the coarsest
 * one that still spends most of the axis on the data.
 */
const LADDERS: readonly (readonly number[])[] = [
	[1, 2, 5],
	[1, 1.5, 2, 3, 5, 7],
	[1, 2, 3, 4, 5, 6, 7, 8, 9],
];

/** The coarsest ladder is the default shape of an axis; the rest are for data that clusters. */
const STEPS = LADDERS[0];

/**
 * How much of an axis its data should occupy before a coarser ladder is preferred for its rounder
 * numbers. Half is enough to read a distribution; below that the marks start to merge.
 */
const MIN_OCCUPANCY = 0.5;

export interface RailTick {
	value: number;
	/** Position along the rail, 0–1. */
	at: number;
	label: string;
}

export interface Rail {
	/** Lower bound of the axis, on the 1-2-5 ladder at or below the smallest value. */
	lo: number;
	/** Upper bound, on the ladder at or above the largest. */
	hi: number;
	ticks: RailTick[];
	/**
	 * Where a value sits, 0–1, or `null` when it cannot be placed: a log axis has no position for
	 * zero or for a negative number, and inventing one would put a mark under a row that was never
	 * measured.
	 */
	at(value: number): number | null;
	/** How many times the largest value exceeds the smallest, e.g. 12.8 for "12.8x apart". */
	spread: number;
}

/** The largest ladder value at or below `v`. */
function ladderBelow(v: number, steps: readonly number[] = STEPS): number {
	const decade = 10 ** Math.floor(Math.log10(v));
	for (const step of [...steps].reverse()) {
		if (step * decade <= v) return step * decade;
	}
	return (steps[steps.length - 1] * decade) / 10;
}

/** The smallest ladder value at or above `v`. */
function ladderAbove(v: number, steps: readonly number[] = STEPS): number {
	const decade = 10 ** Math.floor(Math.log10(v));
	for (const step of steps) {
		if (step * decade >= v) return step * decade;
	}
	return 10 * decade;
}

/** Every ladder value in `[lo, hi]`, ascending. */
function ladderBetween(lo: number, hi: number, steps: readonly number[] = STEPS): number[] {
	const out: number[] = [];
	let decade = 10 ** Math.floor(Math.log10(lo)) / 10;
	while (decade <= hi * 10) {
		for (const step of steps) {
			const value = step * decade;
			if (value >= lo && value <= hi) out.push(value);
		}
		decade *= 10;
	}
	return [...new Set(out)].sort((a, b) => a - b);
}

/**
 * The bounds to draw, and the ladder they came from.
 *
 * Walks the ladders coarsest first and takes the first whose bounds leave the data occupying at
 * least `MIN_OCCUPANCY` of the axis, so round numbers win wherever they can be afforded. The finest
 * ladder is the floor: values that sit within a few percent of each other cannot be bracketed
 * tightly by any set of round numbers, and inventing a bound off the ladder to chase them would
 * label the axis with numbers nobody recognises.
 */
function bracket(min: number, max: number): { lo: number; hi: number; steps: readonly number[] } {
	const span = Math.log10(max) - Math.log10(min);
	let fallback: { lo: number; hi: number; steps: readonly number[] } | null = null;

	for (const steps of LADDERS) {
		const lo = ladderBelow(min, steps);
		const hi = ladderAbove(max, steps);
		const axis = Math.log10(hi) - Math.log10(lo);
		fallback = { lo, hi, steps };
		if (axis <= 0 || span / axis >= MIN_OCCUPANCY) return fallback;
	}
	return fallback as { lo: number; hi: number; steps: readonly number[] };
}

/**
 * Thin a tick list down to at most `max` entries, in three passes: the whole ladder, then powers of
 * ten only, then every other decade.
 *
 * Ticks are dropped rather than crowded because the axis is drawn once at the top of a long table
 * and read from a distance. Six labels across 1,200px is a scale; fourteen is a ruler nobody reads.
 */
function thin(values: number[], max: number): number[] {
	if (values.length <= max) return values;

	const decades = values.filter((v) => isPowerOfTen(v));
	if (decades.length <= max && decades.length >= 2) return decades;

	const stride = Math.ceil(decades.length / max);
	const every = decades.filter((_, i) => i % stride === 0);
	return every.length >= 2 ? every : values.slice(0, max);
}

function isPowerOfTen(v: number): boolean {
	const e = Math.log10(v);
	return Math.abs(e - Math.round(e)) < 1e-9;
}

/**
 * Build a rail over the values currently on screen.
 *
 * The domain comes from the rows in view rather than from every row that exists, for the same
 * reason the old bar scale did: filtering to one database should re-scale the axis to that
 * database's field, not leave it stretched to accommodate rows that are no longer shown. The
 * printed number beside each dot is always the real one, so the axis is a shape cue and never the
 * source of a value.
 */
export function buildRail(
	values: readonly number[],
	format: (value: number) => string,
	maxTicks = 6,
): Rail {
	const finite = values.filter((v) => Number.isFinite(v) && v > 0);

	// Nothing placeable. A rail with no domain still answers `at()` — with null, every time.
	if (finite.length === 0) {
		return { lo: 1, hi: 10, ticks: [], at: () => null, spread: 1 };
	}

	const min = Math.min(...finite);
	const max = Math.max(...finite);

	// One distinct value, or a span too narrow to draw: give it a decade of room so the single dot
	// lands mid-rail instead of on an edge, where it would read as a floor or a ceiling.
	// One distinct value has no span to fit, so it keeps the decade of room that puts its single
	// mark mid-rail rather than against an edge.
	const bounds =
		min === max
			? { lo: ladderBelow(min / 2), hi: ladderAbove(max * 2), steps: STEPS }
			: bracket(min, max);
	const { lo, hi } = bounds;

	const logLo = Math.log10(lo);
	const span = Math.log10(hi) - logLo;

	const at = (value: number): number | null => {
		if (!Number.isFinite(value) || value <= 0) return null;
		if (span <= 0) return 0.5;
		return Math.min(1, Math.max(0, (Math.log10(value) - logLo) / span));
	};

	const ticks = thin(ladderBetween(lo, hi, bounds.steps), maxTicks).map((value) => ({
		value,
		at: at(value) ?? 0,
		label: format(value),
	}));

	return { lo, hi, ticks, at, spread: max / min };
}

/** A rail position as a CSS percentage, for `left:` on the mark. */
export function railLeft(at: number | null): string | null {
	return at === null ? null : `${(at * 100).toFixed(2)}%`;
}
