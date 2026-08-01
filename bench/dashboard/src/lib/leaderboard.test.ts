import { describe, expect, it } from 'vitest';
import { deltaDirection, deltaSentence, rowDelta } from './leaderboard';

/**
 * These pin the one thing about deltas that is easy to get backwards and impossible to spot in a
 * screenshot: which way the sign points.
 *
 * The bug they exist to prevent shipped once. The delta was computed as "how far ahead drizzle is"
 * — `(reference - row) / row` — and then printed on the row, under a column headed "vs drizzle-rs".
 * A Turso baseline measured at 468 rps against drizzle's 469 rendered `+0.3%` on its own row, which
 * reads as "this one is faster". It is slower. The arithmetic was fine; the subject of the sentence
 * was wrong.
 */
describe('rowDelta', () => {
	describe('higher-is-better metrics (throughput)', () => {
		it('is negative when the row is slower than the reference', () => {
			// The exact regression: 468 rps against drizzle's 469.
			const delta = rowDelta(468, 469, true);
			expect(delta).not.toBeNull();
			expect(delta!).toBeLessThan(0);
			expect((delta! * 100).toFixed(1)).toBe('-0.2');
		});

		it('is positive when the row is faster than the reference', () => {
			const delta = rowDelta(600, 500, true)!;
			expect(delta).toBeCloseTo(0.2, 10);
		});

		it('is zero when the row matches the reference', () => {
			expect(rowDelta(500, 500, true)).toBe(0);
		});
	});

	describe('lower-is-better metrics (latency, cpu, memory, errors)', () => {
		it('is positive when the row is faster, so positive still means better', () => {
			// 8ms against a 10ms reference is an improvement, and must not read as -20%.
			const delta = rowDelta(8, 10, false)!;
			expect(delta).toBeGreaterThan(0);
			expect(delta).toBeCloseTo(0.2, 10);
		});

		it('is negative when the row is slower', () => {
			expect(rowDelta(12, 10, false)!).toBeCloseTo(-0.2, 10);
		});
	});

	describe('degenerate inputs', () => {
		it('returns null rather than Infinity when the reference is zero', () => {
			expect(rowDelta(5, 0, true)).toBeNull();
		});

		it('returns null for non-finite inputs', () => {
			expect(rowDelta(Number.NaN, 10, true)).toBeNull();
			expect(rowDelta(10, Number.POSITIVE_INFINITY, true)).toBeNull();
		});

		it('handles a zero row value without dividing by it', () => {
			// A target that recorded no throughput is 100% worse, not a crash or a null.
			expect(rowDelta(0, 500, true)).toBe(-1);
		});
	});
});

describe('deltaDirection', () => {
	it('points up when the row is better and down when it is worse', () => {
		expect(deltaDirection(rowDelta(600, 500, true))).toBe('up');
		expect(deltaDirection(rowDelta(400, 500, true))).toBe('down');
	});

	it('points up for a faster row on a lower-is-better metric', () => {
		expect(deltaDirection(rowDelta(8, 10, false))).toBe('up');
	});

	it('is flat inside the noise threshold, so 0.2% is not drawn as a win', () => {
		expect(deltaDirection(rowDelta(468, 469, true))).toBe('flat');
		expect(deltaDirection(null)).toBe('flat');
	});
});

describe('deltaSentence', () => {
	it('agrees with the sign for higher-is-better metrics', () => {
		const words = { better: 'faster', worse: 'slower' };
		expect(deltaSentence(rowDelta(400, 500, true)!, 'This library', 'drizzle-rs', words)).toBe(
			'This library is 20.0% slower than drizzle-rs',
		);
		expect(deltaSentence(rowDelta(600, 500, true)!, 'This library', 'drizzle-rs', words)).toBe(
			'This library is 20.0% faster than drizzle-rs',
		);
	});

	it('agrees with the sign for lower-is-better metrics', () => {
		const words = { better: 'lower', worse: 'higher' };
		expect(deltaSentence(rowDelta(8, 10, false)!, 'This run', 'the previous run', words)).toBe(
			'This run is 20.0% lower than the previous run',
		);
	});

	it('says "matches" rather than a signed zero inside the noise threshold', () => {
		expect(
			deltaSentence(rowDelta(468, 469, true)!, 'This library', 'drizzle-rs', {
				better: 'faster',
				worse: 'slower',
			}),
		).toBe('This library matches drizzle-rs');
	});
});
