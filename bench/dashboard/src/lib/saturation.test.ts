import { describe, expect, it } from 'vitest';
import { harnessRows, mergeHarness } from './harness';
import { parseRankingSort } from './ranking';
import { anyMeasured, buildCurve, capacity, compareCapacity, readSaturation } from './saturation';
import type { HarnessFamily, SaturationDoc, SaturationStep, Summary } from './types';

/**
 * The rules that make the capacity number honest.
 *
 * Every case here is a way the UI could quietly lie — by reading a paced number as a peak, by
 * ranking a lower bound above a measurement, by rendering a figure without the objective it was
 * measured at, or by inheriting a harness from a neighbouring family. None of those are visible in
 * a screenshot; all of them are wrong on every row.
 */

const SLO = { metric: 'p99', ms: 50 };

function step(
	over: Partial<SaturationStep> & { concurrency: number; rps: number },
): SaturationStep {
	return {
		latency: { p50: 1, p90: 2, p95: 3, p99: 9 },
		err: 0,
		cpu: 40,
		slo_met: true,
		disqualified: null,
		...over,
	};
}

function summary(saturation: Summary['saturation']): Summary {
	return {
		version: 'v1',
		run_id: 'r',
		suite: 'throughput-http',
		target_id: 't',
		primary: {
			rps: { avg: 470, peak: 700 },
			latency: { avg: 2, p95: 5, p99: 9, p999: 20 },
			cpu: { avg: 50, peak: 90 },
			err: 0,
		},
		spread: {
			trials: 3,
			aggregate: 'median',
			rps: { min: 460, max: 480 },
			p95: { min: 4, max: 6 },
			variance: {
				rps: { value: 0, stdev: 0, samples: 3 },
				p95: { value: 0, stdev: 0, samples: 3 },
				cpu: { value: 0, stdev: 0, samples: 3 },
				err: { value: 0, stdev: 0, samples: 3 },
			},
		},
		saturation,
	};
}

const saturated: SaturationDoc = {
	slo: SLO,
	outcome: 'saturated',
	peak: {
		concurrency: 64,
		rps: 12450,
		latency: { p50: 1.2, p90: 3, p95: 4.1, p99: 9.8 },
		cpu: 71.2,
		err: 0,
	},
	curve: [
		step({ concurrency: 8, rps: 1000 }),
		step({ concurrency: 64, rps: 12450 }),
		step({
			concurrency: 128,
			rps: 12100,
			slo_met: false,
			latency: { p50: 4, p90: 40, p95: 60, p99: 130 },
		}),
		step({
			concurrency: 256,
			rps: 13900,
			slo_met: false,
			err: 0.032,
			disqualified: 'error rate 3.2% exceeds limit 1%',
		}),
	],
};

const didNotSaturate: SaturationDoc = {
	slo: SLO,
	outcome: 'did_not_saturate',
	lower_bound_rps: 40000,
	curve: [step({ concurrency: 8, rps: 9000 }), step({ concurrency: 64, rps: 40000 })],
};

const sloNeverMet: SaturationDoc = {
	slo: SLO,
	outcome: 'slo_never_met',
	curve: [
		step({
			concurrency: 8,
			rps: 300,
			slo_met: false,
			latency: { p50: 40, p90: 90, p95: 110, p99: 180 },
		}),
	],
};

describe('reading the artifact', () => {
	it('never reads the legacy knee heuristic as a saturation measurement', () => {
		// This is the shape every published run carries today. It is a paced-suite number, and it
		// falls back to the busiest bucket when it finds no knee — so treating it as a peak would be
		// the exact substitution the design forbids.
		const legacy = summary({ knee_rps: 464.9, knee_p95: 2.1 });
		expect(readSaturation(legacy)).toBeNull();
		expect(capacity(legacy).state).toBe('not-measured');
		expect(capacity(legacy).figure).toBeNull();
		expect(anyMeasured([legacy])).toBe(false);
	});

	it('treats an absent block as not measured rather than zero', () => {
		const view = capacity(summary(undefined));
		expect(view.state).toBe('not-measured');
		expect(view.figure).toBeNull();
		expect(view.note).toBe('not measured');
	});

	it('refuses an outcome it does not know rather than guessing at one', () => {
		const alien = summary({ slo: SLO, outcome: 'melted', curve: [] } as unknown as SaturationDoc);
		expect(readSaturation(alien)).toBeNull();
	});
});

describe('the figure and its qualifier', () => {
	it('carries the objective with every number it reports', () => {
		// The invariant the whole design rests on: there is no state that produces a bare number.
		for (const doc of [saturated, didNotSaturate, sloNeverMet, undefined]) {
			const view = capacity(summary(doc));
			if (view.figure) expect(view.figure.qualifier).toBe('at p99 < 50 ms');
		}
	});

	it('reports a measured peak with its concurrency', () => {
		const view = capacity(summary(saturated));
		expect(view.state).toBe('measured');
		expect(view.figure?.text).toBe('12.4k');
		expect(view.figure?.lowerBound).toBe(false);
		expect(view.note).toBe('at 64 concurrent');
		expect(view.rankable).toBe(true);
	});

	it('says "at least" and "knee not reached" instead of presenting a peak', () => {
		const view = capacity(summary(didNotSaturate));
		expect(view.state).toBe('lower-bound');
		expect(view.figure?.text).toBe('at least 40.0k');
		expect(view.figure?.lowerBound).toBe(true);
		expect(view.note).toBe('knee not reached');
		// A lower bound is never given a rank: it is not a placement.
		expect(view.rankable).toBe(false);
	});

	it('substitutes no number when the objective was never met', () => {
		const view = capacity(summary(sloNeverMet));
		expect(view.state).toBe('never-met');
		expect(view.figure).toBeNull();
		expect(view.note).toBe('never met the p99 target');
		expect(view.tierValue).toBe(0);
	});
});

describe('ordering', () => {
	it('never lets a state without a measured peak outrank one that has it', () => {
		// The lower bound here (40k) is more than three times the measured peak (12.45k) — and still
		// sorts below it. A ramp that stopped early is not evidence of being faster.
		const measured = capacity(summary(saturated));
		const bounded = capacity(summary(didNotSaturate));
		expect(compareCapacity(measured, bounded)).toBeLessThan(0);
	});

	it('orders the four states measured, lower bound, never met, not measured', () => {
		const views = [
			capacity(summary(undefined)),
			capacity(summary(sloNeverMet)),
			capacity(summary(didNotSaturate)),
			capacity(summary(saturated)),
		].sort(compareCapacity);
		expect(views.map((view) => view.state)).toEqual([
			'measured',
			'lower-bound',
			'never-met',
			'not-measured',
		]);
	});
});

describe('the curve', () => {
	it('marks the peak the runner chose and nothing else', () => {
		const curve = buildCurve(saturated);
		expect(curve.peakIndex).toBe(1);
		expect(curve.points.filter((point) => point.isPeak)).toHaveLength(1);
		expect(curve.peakMissing).toBe(false);
	});

	it('keeps a disqualified step on the curve, with its reason', () => {
		const curve = buildCurve(saturated);
		const struck = curve.points.find((point) => point.disqualified !== null);
		expect(struck?.concurrency).toBe(256);
		expect(struck?.verdict).toBe('disqualified');
		expect(struck?.verdictText).toContain('error rate 3.2% exceeds limit 1%');
		expect(curve.disqualifiedCount).toBe(1);
		// It is the fastest step on the curve and it is still not the peak.
		expect(struck?.rps).toBeGreaterThan(saturated.peak!.rps);
		expect(struck?.isPeak).toBe(false);
	});

	it('keeps the objective inside the latency axis so the threshold is always drawn', () => {
		// Every step here is far under the objective; without the floor the rule would sit off-screen.
		expect(buildCurve(didNotSaturate).latencyMax).toBeGreaterThanOrEqual(50);
	});

	it('withholds the peak marker when the artifact names a step that is not in its curve', () => {
		const curve = buildCurve({ ...saturated, peak: { ...saturated.peak!, concurrency: 999 } });
		expect(curve.peakIndex).toBeNull();
		expect(curve.peakMissing).toBe(true);
		expect(curve.points.some((point) => point.isPeak)).toBe(false);
	});

	it('draws a curve even when there is no peak to mark', () => {
		const curve = buildCurve(sloNeverMet);
		expect(curve.points).toHaveLength(1);
		expect(curve.peakIndex).toBeNull();
		expect(curve.points[0].verdict).toBe('over');
	});
});

describe('the sort control', () => {
	it('honours a capacity order on a set that measured capacity', () => {
		expect(parseRankingSort('capacity', 'capacity', ['capacity', 'throughput', 'latency'])).toBe(
			'capacity',
		);
	});

	it('refuses a capacity order on a set that measured none', () => {
		// Honouring it would render a table where every row is unranked and their order carries no
		// meaning — a control that looks like it worked and did nothing.
		expect(parseRankingSort('capacity', 'throughput', ['throughput', 'latency'])).toBe(
			'throughput',
		);
	});

	it('falls back to the set default for an unknown value', () => {
		expect(parseRankingSort('bogus', 'capacity')).toBe('capacity');
		expect(parseRankingSort(null, 'throughput')).toBe('throughput');
	});
});

describe('harness disclosure', () => {
	const family = (over: Partial<HarnessFamily> & { family: string }): HarnessFamily => ({
		workers: 1,
		pool: 8,
		tuning: 'stock',
		within_family_identical: true,
		...over,
	});

	it('states that a family declared nothing rather than borrowing a neighbour', () => {
		const rows = harnessRows(['sqlite', 'postgres'], [family({ family: 'postgres' })], (db) => db);
		const sqlite = rows.find((row) => row.db === 'sqlite');
		expect(sqlite?.summary).toBeNull();
		// Not `false`: "we did not record it" and "we checked and it differs" are different findings.
		expect(sqlite?.identical).toBeNull();
	});

	it('matches families case-insensitively', () => {
		const rows = harnessRows(['postgres'], [family({ family: 'Postgres' })], (db) => db);
		expect(rows[0].summary).toBe('1 worker / pool 8 / stock');
	});

	it('reports shards that disagree instead of picking one', () => {
		const merged = mergeHarness([
			{ harness: [family({ family: 'postgres', pool: 8 })] },
			{ harness: [family({ family: 'postgres', pool: 32 })] },
		]);
		expect(merged.conflicts).toHaveLength(1);
		expect(merged.conflicts[0]).toContain('pool 8');
		expect(merged.conflicts[0]).toContain('pool 32');
		// And the family is downgraded, so the strip cannot claim it was verified.
		expect(merged.harness[0].within_family_identical).toBe(false);
	});

	it('lets one shard reporting the family unverified win over one that did not', () => {
		const merged = mergeHarness([
			{ harness: [family({ family: 'sqlite' })] },
			{ harness: [family({ family: 'sqlite', within_family_identical: false })] },
		]);
		expect(merged.conflicts).toHaveLength(0);
		expect(merged.harness[0].within_family_identical).toBe(false);
	});
});
