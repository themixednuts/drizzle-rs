import { describe, expect, it } from 'vitest';
import { latencyBasis, latencyView, readLatency } from './service-latency';
import type { LatencyOutcome, Summary, SustainedStep } from './types';

function step(concurrency: number, p95: number, retention = 1): SustainedStep {
	return {
		concurrency,
		rps: 1000,
		offered_rps: 1000 / retention,
		retention,
		latency: { p50: p95 / 2, p90: p95 * 0.9, p95, p99: p95 * 1.2 },
		cpu: 50,
		err: 0,
		sustained: retention >= 0.9,
		disqualified: null,
	};
}

function summary(
	latency?: Partial<Summary['latency']>,
	p95 = 2500,
): Pick<Summary, 'latency' | 'primary'> {
	return {
		primary: {
			rps: { avg: 1300, peak: 2000 },
			latency: { avg: p95 / 2, p50: p95 / 3, p90: p95 * 0.8, p95, p99: p95 * 1.3, p999: p95 * 1.5 },
			cpu: { avg: 99, peak: 100 },
			err: 0,
		},
		latency: latency as Summary['latency'],
	};
}

const measured = (reference: SustainedStep) => ({
	tolerance: 0.1,
	outcome: 'measured' as LatencyOutcome,
	reference,
	curve: [reference, step(reference.concurrency * 2, reference.latency.p95 * 4, 0.5)],
});

describe('readLatency', () => {
	it('reads a block that carries a reading', () => {
		expect(readLatency(summary(measured(step(400, 12))))?.reference?.concurrency).toBe(400);
	});

	it('treats an unrecognised outcome as absent rather than guessing', () => {
		// A future runner's outcome must not be rendered as one of the ones this build knows.
		expect(
			readLatency(summary({ tolerance: 0.1, outcome: 'someday' as LatencyOutcome, curve: [] })),
		).toBeNull();
	});

	it('treats a reading-bearing outcome with no reading as malformed', () => {
		expect(readLatency(summary({ tolerance: 0.1, outcome: 'measured', curve: [] }))).toBeNull();
	});

	it('keeps the floor outcomes, which have no figure but are still a finding', () => {
		for (const outcome of ['floor_above_knee', 'floor_disqualified'] as LatencyOutcome[]) {
			const doc = readLatency(summary({ tolerance: 0.1, outcome, curve: [step(25, 9)] }));
			expect(doc?.outcome).toBe(outcome);
			expect(doc?.reference).toBeUndefined();
		}
	});

	it('is absent on every run published before the field existed', () => {
		expect(readLatency(summary(undefined))).toBeNull();
	});
});

describe('latencyBasis', () => {
	it('needs every row to carry a reading', () => {
		expect(latencyBasis([summary(measured(step(400, 12))), summary(measured(step(800, 30)))])).toBe(
			'sustained',
		);
	});

	it('falls to whole-ramp when one row predates the field', () => {
		// The alternative is two different measurements down one column.
		expect(latencyBasis([summary(measured(step(400, 12))), summary(undefined)])).toBe('whole-ramp');
	});

	it('falls to whole-ramp when a row has a block but no figure', () => {
		const noFigure = summary({ tolerance: 0.1, outcome: 'floor_above_knee', curve: [step(25, 9)] });
		expect(latencyBasis([summary(measured(step(400, 12))), noFigure])).toBe('whole-ramp');
	});

	it('is whole-ramp for an empty table', () => {
		expect(latencyBasis([])).toBe('whole-ramp');
	});
});

describe('latencyView', () => {
	it('reports the sustained rung, not the whole-ramp figure', () => {
		const view = latencyView(summary(measured(step(400, 12)), 2500), 'sustained');
		expect(view.basis).toBe('sustained');
		expect(view.value).toBe(12);
		expect(view.note).toBe('at 400 VUs');
	});

	it('reports the whole-ramp figure on the whole-ramp basis even when a reading exists', () => {
		// A row that could show the better number still does not, when its table cannot show it
		// for every row.
		const view = latencyView(summary(measured(step(400, 12)), 2500), 'whole-ramp');
		expect(view.basis).toBe('whole-ramp');
		expect(view.value).toBe(2500);
		expect(view.note).toBe('whole ramp');
	});

	it('falls back rather than blanking when a row cannot honour the basis it was given', () => {
		const view = latencyView(summary(undefined, 2500), 'sustained');
		expect(view.basis).toBe('whole-ramp');
		expect(view.value).toBe(2500);
	});

	it('says how far up the ramp the target went, without reading the figure there', () => {
		const reference = step(50, 4);
		const view = latencyView(
			summary({
				tolerance: 0.1,
				outcome: 'measured',
				reference,
				curve: [reference, step(400, 90, 0.97), step(800, 400, 0.4)],
			}),
			'sustained',
		);
		// The figure stays at the reference rung; how far it held is context, not the reading.
		expect(view.value).toBe(4);
		expect(view.detail).toContain('up to 400 concurrent');
	});

	it('quotes the retention beside the figure, so the reading can be checked', () => {
		const view = latencyView(summary(measured(step(400, 12, 0.94))), 'sustained');
		expect(view.detail).toContain('94.00%');
	});
});
