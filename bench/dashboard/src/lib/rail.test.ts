import { describe, expect, it } from 'vitest';
import { buildRail, railLeft } from './rail';

/**
 * The ratio rail is the one thing on the ranking that turns a number into a position, so every row
 * on the page is wrong together if this is wrong. It is also the part a screenshot cannot check:
 * a mispositioned dot still looks like a dot.
 */

const rps = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(0)}k` : String(n));

describe('buildRail', () => {
	it('puts equal ratios at equal distances', () => {
		const rail = buildRail([100, 1000, 10000], rps);

		const decade = rail.at(1000)! - rail.at(100)!;
		expect(rail.at(10000)! - rail.at(1000)!).toBeCloseTo(decade, 10);
	});

	it('is not distorted by one outlier the way a linear scale is', () => {
		// The real shape of this data: a cache-backed row several times faster than the field.
		const rail = buildRail([767, 858, 3200, 4600, 7600, 9800], rps);

		// On a linear scale 767/9800 puts the slowest row at 7.8% of the width — a stub against the
		// left edge. On the rail it sits at 14.3%, because the field spans about one decade and the
		// axis is scaled to decades rather than to the largest number in it.
		expect(rail.at(767)!).toBeCloseTo(0.143, 2);
		// And the gap between the two slowest rows stays legible rather than collapsing.
		expect(rail.at(858)! - rail.at(767)!).toBeGreaterThan(0.02);
	});

	it('lands its bounds on the 1-2-5 ladder', () => {
		const rail = buildRail([767, 9800], rps);
		expect(rail.lo).toBe(500);
		expect(rail.hi).toBe(10000);
	});

	it('clamps the ends to the axis', () => {
		const rail = buildRail([1000, 10000], rps);
		expect(rail.at(1000)).toBe(0);
		expect(rail.at(10000)).toBe(1);
	});

	it('refuses to place a value a log axis has no room for', () => {
		const rail = buildRail([100, 1000], rps);
		// Zero is not a position on a log scale, and neither is a negative rate. Drawing either at
		// the left edge would put a mark under a row that was never measured.
		expect(rail.at(0)).toBeNull();
		expect(rail.at(-5)).toBeNull();
		expect(rail.at(Number.NaN)).toBeNull();
	});

	it('gives a single value room instead of pinning it to an edge', () => {
		const rail = buildRail([4200], rps);
		expect(rail.at(4200)).toBeGreaterThan(0.1);
		expect(rail.at(4200)).toBeLessThan(0.9);
	});

	it('survives having nothing to place', () => {
		const rail = buildRail([], rps);
		expect(rail.ticks).toEqual([]);
		expect(rail.at(100)).toBeNull();
	});

	it('reports how far apart the field is', () => {
		expect(buildRail([767, 9800], rps).spread).toBeCloseTo(12.78, 2);
	});

	describe('ticks', () => {
		it('stay within the axis and in order', () => {
			const rail = buildRail([767, 9800], rps);
			expect(rail.ticks.length).toBeGreaterThan(1);
			for (const tick of rail.ticks) {
				expect(tick.value).toBeGreaterThanOrEqual(rail.lo);
				expect(tick.value).toBeLessThanOrEqual(rail.hi);
			}
			const positions = rail.ticks.map((t) => t.at);
			expect(positions).toEqual([...positions].sort((a, b) => a - b));
		});

		it('thin out rather than crowd when the span is wide', () => {
			// Latency across this suite: 1.8ms to 4.78s, nearly four decades.
			const rail = buildRail([1.8, 126, 415, 4780], (n) => `${n}ms`, 6);
			expect(rail.ticks.length).toBeLessThanOrEqual(6);
			expect(rail.ticks.length).toBeGreaterThanOrEqual(2);
		});

		it('are labelled by the caller, so an axis speaks its metric', () => {
			const rail = buildRail([100, 10000], rps);
			expect(rail.ticks.map((t) => t.label)).toContain('10k');
		});
	});
});

describe('railLeft', () => {
	it('renders a position as a percentage', () => {
		expect(railLeft(0.5)).toBe('50.00%');
		expect(railLeft(0)).toBe('0.00%');
	});

	it('passes null through, so an unplaceable row draws nothing', () => {
		expect(railLeft(null)).toBeNull();
	});
});

describe('bracketing a narrow range', () => {
	/** How much of the axis the data actually occupies. */
	function occupancy(values: number[]): number {
		const rail = buildRail(values, (v) => String(v));
		const lo = Math.min(...values);
		const hi = Math.max(...values);
		return (Math.log10(hi) - Math.log10(lo)) / (Math.log10(rail.hi) - Math.log10(rail.lo));
	}

	it('does not spend the axis on empty space when the values cluster', () => {
		// A preview run's request rates. On the 1-2-5 ladder alone these snap to 200..500 and draw as
		// one clump against the right edge, using an eighth of the width.
		expect(occupancy([440, 455, 470, 492])).toBeGreaterThanOrEqual(0.5);
	});

	it('still prefers the round 1-2-5 bounds when the data spans decades', () => {
		const rail = buildRail([766, 1300, 4600, 9800], (v) => String(v));
		expect([rail.lo, rail.hi]).toEqual([500, 10000]);
	});

	it('keeps every value inside the axis it chose', () => {
		for (const values of [
			[440, 492],
			[0.3, 56],
			[766, 9800],
			[1.8, 2.1],
			[12, 13],
		]) {
			const rail = buildRail(values, (v) => String(v));
			for (const v of values) {
				expect(rail.at(v)).toBeGreaterThanOrEqual(0);
				expect(rail.at(v)).toBeLessThanOrEqual(1);
			}
			expect(rail.lo).toBeLessThanOrEqual(Math.min(...values));
			expect(rail.hi).toBeGreaterThanOrEqual(Math.max(...values));
		}
	});

	it('puts its ticks on the ladder it bracketed with', () => {
		const rail = buildRail([440, 492], (v) => String(v));
		expect(rail.ticks.length).toBeGreaterThanOrEqual(2);
		for (const tick of rail.ticks) {
			expect(tick.value).toBeGreaterThanOrEqual(rail.lo);
			expect(tick.value).toBeLessThanOrEqual(rail.hi);
		}
	});
});
