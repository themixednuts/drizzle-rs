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

/** The 1-2-5 ladder. Bounds and ticks both land on it, so an axis never ends on an odd number. */
const STEPS = [1, 2, 5] as const;

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
function ladderBelow(v: number): number {
	const decade = 10 ** Math.floor(Math.log10(v));
	for (const step of [5, 2, 1] as const) {
		if (step * decade <= v) return step * decade;
	}
	return decade / 2;
}

/** The smallest ladder value at or above `v`. */
function ladderAbove(v: number): number {
	const decade = 10 ** Math.floor(Math.log10(v));
	for (const step of STEPS) {
		if (step * decade >= v) return step * decade;
	}
	return 10 * decade;
}

/** Every ladder value in `[lo, hi]`, ascending. */
function ladderBetween(lo: number, hi: number): number[] {
	const out: number[] = [];
	let decade = 10 ** Math.floor(Math.log10(lo));
	while (decade <= hi * 10) {
		for (const step of STEPS) {
			const value = step * decade;
			if (value >= lo && value <= hi) out.push(value);
		}
		decade *= 10;
	}
	return out;
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
	const lo = min === max ? ladderBelow(min / 2) : ladderBelow(min);
	const hi = min === max ? ladderAbove(max * 2) : ladderAbove(max);

	const logLo = Math.log10(lo);
	const span = Math.log10(hi) - logLo;

	const at = (value: number): number | null => {
		if (!Number.isFinite(value) || value <= 0) return null;
		if (span <= 0) return 0.5;
		return Math.min(1, Math.max(0, (Math.log10(value) - logLo) / span));
	};

	const ticks = thin(ladderBetween(lo, hi), maxTicks).map((value) => ({
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
