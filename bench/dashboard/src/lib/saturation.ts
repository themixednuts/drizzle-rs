import { fmtCpu, fmtLatency, fmtPct, fmtRps } from './format';
import type { SaturationDoc, SaturationSlo, SaturationStep, Summary } from './types';

/**
 * Peak throughput, and the three ways a target can fail to have one.
 *
 * This module is the whole vocabulary of the saturation suite, in one place, because the rule it
 * enforces is a rule about *substitution*: when the peak is unavailable no other number may stand
 * in for it. That is only checkable if there is exactly one function that decides what a target's
 * capacity figure is, and exactly one shape a caller can render — hence `capacity()` returning a
 * `CapacityView` whose number and whose qualifier live in the same object, and `CapacityFigure`
 * being the only component that draws them.
 *
 * Two numbers on this site are throughput and they mean different things:
 *
 *   - **peak throughput** — the saturation suite's unpaced closed-loop ramp, reported as the
 *     highest concurrency step that held the latency objective. This is a capacity claim.
 *   - **throughput at fixed load** — the paced suite's request rate. With per-request think time
 *     the generator cannot offer more than `VUs / think time`, so a healthy target reports the
 *     pacing ceiling. This is a latency-at-a-known-rate claim and is not capacity.
 *
 * They are never averaged, never compared, and never rendered with the same words.
 */

// -- Reading the artifact ------------------------------------------------------------------------

/**
 * The saturation block, or `null` when the run did not measure one.
 *
 * `outcome` is the discriminator and the only one. Artifacts published before the saturation suite
 * carry a *differently shaped* `saturation` object — the legacy `knee_rps`/`knee_p95` heuristic
 * computed off the paced run, which always produces a number because it falls back to the
 * highest-throughput bucket when it finds no knee. Reading that as a peak would be precisely the
 * relabelled-paced-number failure this design exists to prevent, so anything without an `outcome`
 * is "not measured" and is never read again.
 */
export function readSaturation(summary: Pick<Summary, 'saturation'>): SaturationDoc | null {
	const raw = summary.saturation;
	if (!raw || !('outcome' in raw)) return null;

	switch (raw.outcome) {
		case 'saturated':
			return raw.peak ? raw : null;
		case 'did_not_saturate':
			return typeof raw.lower_bound_rps === 'number' ? raw : null;
		case 'slo_never_met':
			return raw;
		default:
			// An outcome this build does not know is not something to guess at. Rendering it as one of
			// the three we do know would be inventing a finding.
			return null;
	}
}

/** True when at least one target in the set carries a saturation measurement. */
export function anyMeasured(summaries: readonly Pick<Summary, 'saturation'>[]): boolean {
	return summaries.some((summary) => readSaturation(summary) !== null);
}

// -- The figure ----------------------------------------------------------------------------------

/**
 * How a target's capacity is known.
 *
 * - `measured` — a real peak. The only state that carries a comparable number.
 * - `lower-bound` — every step held the objective, so the ramp stopped short of the knee. The
 *   number is a floor, not a measurement.
 * - `never-met` — even the smallest step breached the objective. There is no peak.
 * - `not-measured` — this run did not run the saturation suite for this target.
 */
export type CapacityState = 'measured' | 'lower-bound' | 'never-met' | 'not-measured';

/**
 * A capacity number and the objective it was measured against, as one indivisible value.
 *
 * The qualifier is not an optional decoration on the number — it is what the number means. Keeping
 * them in the same object is what makes "a bare rps figure a reader could mistake for the paced
 * number" unrepresentable: there is no `text` without a `qualifier` to render beside it.
 */
export interface CapacityFigure {
	/** The number, formatted. Reads "12.5k" when measured, "at least 12.5k" when a lower bound. */
	text: string;
	/** Always rendered adjacent to `text`, e.g. "at p99 < 50 ms". */
	qualifier: string;
	/** True when `text` is a floor rather than a measurement; drives the muted, marked treatment. */
	lowerBound: boolean;
}

export interface CapacityView {
	state: CapacityState;
	/** `null` in every state that has no number. Nothing is ever substituted for it. */
	figure: CapacityFigure | null;
	/**
	 * The short words that carry the state: "at 64 concurrent", "knee not reached",
	 * "never met the p99 target", "not measured".
	 */
	note: string;
	/** The full explanation, for a tooltip or a detail line. */
	detail: string;
	/** Sort bucket. Ascending, so a measured peak always precedes every state that lacks one. */
	tier: number;
	/** Ordering inside the bucket, descending. Zero wherever there is no number. */
	tierValue: number;
	/**
	 * Whether this row may carry a rank number in a capacity-ordered table. Only a measured peak
	 * can: a rank on a lower bound or on an unmeasured row would be a position the data does not
	 * support, and position is exactly what readers trust most.
	 */
	rankable: boolean;
}

const TIERS: Record<CapacityState, number> = {
	measured: 0,
	'lower-bound': 1,
	'never-met': 2,
	'not-measured': 3,
};

/** "50 ms", or "12.5 ms" when the objective is not a whole millisecond. */
export function fmtSloMs(ms: number): string {
	return `${Number.isInteger(ms) ? ms : ms.toFixed(1)} ms`;
}

/** The words that must sit beside every capacity number: "at p99 < 50 ms". */
export function sloQualifier(slo: SaturationSlo): string {
	return `at ${slo.metric} < ${fmtSloMs(slo.ms)}`;
}

/** How the objective is named on its own, e.g. for a chart rule: "p99 < 50 ms". */
export function sloLabel(slo: SaturationSlo): string {
	return `${slo.metric} < ${fmtSloMs(slo.ms)}`;
}

const NOT_MEASURED: CapacityView = {
	state: 'not-measured',
	figure: null,
	note: 'not measured',
	detail:
		'This run published no saturation measurement for this target, so it has no peak throughput. The throughput figure beside it comes from the paced suite, which measures latency at a fixed request rate and cannot produce a capacity number.',
	tier: TIERS['not-measured'],
	tierValue: 0,
	rankable: false,
};

/** The one place a target's capacity figure is decided. */
export function capacity(summary: Pick<Summary, 'saturation'>): CapacityView {
	const doc = readSaturation(summary);
	if (!doc) return NOT_MEASURED;

	const qualifier = sloQualifier(doc.slo);
	const objective = sloLabel(doc.slo);

	if (doc.outcome === 'saturated') {
		const peak = doc.peak;
		return {
			state: 'measured',
			figure: { text: fmtRps(peak.rps), qualifier, lowerBound: false },
			note: `at ${peak.concurrency} concurrent`,
			detail: `Peak throughput ${fmtRps(peak.rps)} req/s ${qualifier}, reached at ${peak.concurrency} concurrent requests (${doc.slo.metric} ${fmtLatency(peak.latency.p99)}, cpu ${fmtCpu(peak.cpu)}, errors ${fmtPct(peak.err)}). A higher concurrency step breached ${objective}, which is what makes this a measured peak rather than the top of the ramp.`,
			tier: TIERS.measured,
			tierValue: peak.rps,
			rankable: true,
		};
	}

	if (doc.outcome === 'did_not_saturate') {
		const bound = doc.lower_bound_rps;
		return {
			state: 'lower-bound',
			figure: { text: `at least ${fmtRps(bound)}`, qualifier, lowerBound: true },
			note: 'knee not reached',
			detail: `Every concurrency step held ${objective}, so the ramp never found the knee. ${fmtRps(bound)} req/s is a lower bound, not a peak: this target sustains at least that much and may sustain considerably more. The ramp needs extending before a peak can be claimed.`,
			tier: TIERS['lower-bound'],
			tierValue: bound,
			rankable: false,
		};
	}

	return {
		state: 'never-met',
		figure: null,
		note: `never met the ${doc.slo.metric} target`,
		detail: `Even the smallest concurrency step breached ${objective}, so this target has no peak throughput at this objective. No number is reported in place of one.`,
		tier: TIERS['never-met'],
		tierValue: 0,
		rankable: false,
	};
}

/**
 * Capacity ordering.
 *
 * A row without a measured peak never outranks one that has it, whatever number it happens to
 * carry: a lower bound of 40k is not evidence of beating a measured 12k, it is evidence that the
 * ramp stopped early. Sorting them together would turn "we did not find out" into a placement. So
 * the tiers are hard, the state is printed on the row rather than implied by where the row sits,
 * and only tier 0 is given a rank number at all.
 */
export function compareCapacity(a: CapacityView, b: CapacityView): number {
	if (a.tier !== b.tier) return a.tier - b.tier;
	return b.tierValue - a.tierValue;
}

// -- The curve -----------------------------------------------------------------------------------

/** Why a step is or is not the headline, in the words the table and the tooltip both use. */
export type StepVerdict = 'peak' | 'under' | 'over' | 'disqualified';

export interface CurvePoint {
	/** Position on an evenly-spaced ordinal axis. Concurrency ramps are geometric, so plotting
	 * concurrency linearly would crowd every early step into the first few pixels. */
	index: number;
	concurrency: number;
	rps: number;
	p50: number;
	p90: number;
	p95: number;
	p99: number;
	err: number;
	cpu: number;
	sloMet: boolean;
	disqualified: string | null;
	isPeak: boolean;
	verdict: StepVerdict;
	/** Plain-language reading of `verdict`, carrying the disqualification reason when there is one. */
	verdictText: string;
}

export interface CurveView {
	points: CurvePoint[];
	slo: SaturationSlo;
	/** "p99 < 50 ms", for the threshold rule's label. */
	sloLabel: string;
	/** Top of the throughput axis, already headroomed. */
	rpsMax: number;
	/** Top of the latency axis. Always at least the objective, so the rule is never off-screen. */
	latencyMax: number;
	/** Index of the peak step, or `null` when there is no peak to mark. */
	peakIndex: number | null;
	/**
	 * Set when the artifact claims a peak whose concurrency is absent from the curve. The chart
	 * says so rather than marking the nearest step and calling it the peak.
	 */
	peakMissing: boolean;
	/** How many steps carry a disqualification reason. */
	disqualifiedCount: number;
}

function verdictOf(step: SaturationStep, isPeak: boolean): { verdict: StepVerdict; text: string } {
	if (step.disqualified !== null) {
		return { verdict: 'disqualified', text: `disqualified — ${step.disqualified}` };
	}
	if (isPeak) return { verdict: 'peak', text: 'peak — highest step that held the objective' };
	if (step.slo_met) return { verdict: 'under', text: 'held the objective' };
	return { verdict: 'over', text: 'breached the objective' };
}

/** Headroom above the tallest mark, so a peak does not sit flush against the frame. */
const AXIS_HEADROOM = 1.08;

export function buildCurve(doc: SaturationDoc): CurveView {
	const steps = [...doc.curve].sort((a, b) => a.concurrency - b.concurrency);
	const peakConcurrency = doc.outcome === 'saturated' ? doc.peak.concurrency : null;
	// A peak whose concurrency is not in the curve is marked as missing rather than snapped to the
	// nearest step: labelling a step "peak" that the runner did not choose invents a finding.
	const found =
		peakConcurrency === null ? -1 : steps.findIndex((step) => step.concurrency === peakConcurrency);
	const resolvedPeak = found === -1 ? null : found;

	const points = steps.map((step, index) => {
		const isPeak = index === resolvedPeak;
		const { verdict, text } = verdictOf(step, isPeak);
		return {
			index,
			concurrency: step.concurrency,
			rps: step.rps,
			p50: step.latency.p50,
			p90: step.latency.p90,
			p95: step.latency.p95,
			p99: step.latency.p99,
			err: step.err,
			cpu: step.cpu,
			sloMet: step.slo_met,
			disqualified: step.disqualified,
			isPeak,
			verdict,
			verdictText: text,
		} satisfies CurvePoint;
	});

	const rpsCeiling = Math.max(1, ...points.map((point) => point.rps));
	const latencyCeiling = Math.max(doc.slo.ms, ...points.map((point) => point.p99));

	return {
		points,
		slo: doc.slo,
		sloLabel: sloLabel(doc.slo),
		rpsMax: rpsCeiling * AXIS_HEADROOM,
		latencyMax: latencyCeiling * AXIS_HEADROOM,
		peakIndex: resolvedPeak,
		peakMissing: peakConcurrency !== null && resolvedPeak === null,
		disqualifiedCount: points.filter((point) => point.disqualified !== null).length,
	};
}
