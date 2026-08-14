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
	/** Sort bucket. Ascending, so every state carrying a number precedes every state without one. */
	tier: number;
	/** Ordering inside the bucket, descending. Zero wherever there is no number. */
	tierValue: number;
	/**
	 * Whether this row may carry a rank number in a capacity-ordered table.
	 *
	 * A measured peak and a lower bound both can; a row with no number at all cannot, because a
	 * position is exactly what readers trust most and there is nothing behind it.
	 *
	 * Lower bounds were excluded once, on the reasoning that a rank they cannot support is worse
	 * than no rank. That was half right and cost more than it saved. A lower bound is not unordered
	 * against everything: a target that sustains *at least* 75.9k certainly beats one whose peak was
	 * measured at 47.2k, and excluding it dropped the fastest targets out of the ranking entirely
	 * while sorting them beneath rows they provably beat. Ordering on the printed figure places each
	 * bound at its lowest possible position — every certain relation is honoured, every uncertain one
	 * resolves downward, and the row can never be overstated. The figure keeps its "at least" either
	 * way; `CapacityFigure` is the only thing that can draw it and never draws it bare.
	 */
	rankable: boolean;
}

/**
 * Two buckets, not four: rows that carry a number and rows that do not.
 *
 * A measured peak and a lower bound share the first, because both can be placed on the axis and
 * the ordering below resolves their uncertainty conservatively. `never-met` and `not-measured`
 * have no number at all, so they sort under everything and take no rank.
 */
const TIERS: Record<CapacityState, number> = {
	measured: 0,
	'lower-bound': 0,
	'never-met': 1,
	'not-measured': 2,
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
			// Two things are true of this outcome and the copy has to carry both: the ramp stayed
			// inside the objective, *and* throughput never turned over. It used to say only the
			// first and conclude "the ramp needs extending", which is the wrong inference — a
			// closed-loop target at its ceiling holds throughput flat however far the ramp runs, so
			// more rungs re-confirm a plateau rather than finding a knee that is not there.
			detail: `Throughput was still flat or climbing when the ramp ran out, and every step held ${objective}. ${fmtRps(bound)} req/s is a floor rather than a peak: this target sustains at least that much, and where it stops was not measured. It is ranked on that floor, which is the lowest place its own data allows — above every peak it certainly beats, below the ones it may or may not.`,
			tier: TIERS['lower-bound'],
			tierValue: bound,
			rankable: true,
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
 * Rows carrying a number sort against each other on that number, descending; rows carrying none
 * sort beneath all of them. A lower bound is ordered on its floor, which places it at the lowest
 * position its data allows: it lands above every peak it certainly beats, and below the ones whose
 * relation to it is unknown. That is the conservative half of a partial order, and it cannot
 * overstate a row.
 *
 * The state is still printed on the row rather than implied by where the row sits, because where a
 * bound sits is a floor rather than a placement, and only the words say so.
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
	/**
	 * A step that beat the marked peak on throughput while also qualifying, when one exists.
	 *
	 * The peak is the qualifying step with the highest throughput, so on a well-formed artifact
	 * this is always `null` — the marked point IS the tallest qualifying point. It is computed
	 * anyway because artifacts selected under the earlier "highest qualifying concurrency" rule
	 * exist and still render here: on a ramp that dips once the pool saturates, those name a peak
	 * several percent below the tallest qualifying step, and a reader looking at the chart sees the
	 * discrepancy immediately. Same class of disclosure as `peakMissing` — the artifact disagrees
	 * with itself, and the UI says so instead of letting the mark quietly contradict the line.
	 */
	tallerThanPeak: { concurrency: number; rps: number } | null;
}

function verdictOf(step: SaturationStep, isPeak: boolean): { verdict: StepVerdict; text: string } {
	if (step.disqualified !== null) {
		return { verdict: 'disqualified', text: `disqualified — ${step.disqualified}` };
	}
	if (isPeak) {
		return { verdict: 'peak', text: 'peak — fastest step that held the objective' };
	}
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

	// Only a step that is eligible to be the peak counts here — one that held the objective and was
	// not disqualified. A disqualified step being taller is already explained by its own strike and
	// its reason; saying "this one was faster" about a step that blew the error budget would be
	// pointing at the wrong thing.
	const peakPoint = resolvedPeak === null ? null : points[resolvedPeak];
	const tallest = points
		.filter((point) => point.sloMet && point.disqualified === null)
		.reduce<CurvePoint | null>(
			(best, point) => (best === null || point.rps > best.rps ? point : best),
			null,
		);

	return {
		points,
		slo: doc.slo,
		sloLabel: sloLabel(doc.slo),
		rpsMax: rpsCeiling * AXIS_HEADROOM,
		latencyMax: latencyCeiling * AXIS_HEADROOM,
		peakIndex: resolvedPeak,
		peakMissing: peakConcurrency !== null && resolvedPeak === null,
		disqualifiedCount: points.filter((point) => point.disqualified !== null).length,
		tallerThanPeak:
			peakPoint && tallest && tallest.rps > peakPoint.rps
				? { concurrency: tallest.concurrency, rps: tallest.rps }
				: null,
	};
}
