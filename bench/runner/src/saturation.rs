//! Readings off the per-step curve of a concurrency ramp.
//!
//! Two measurements live here because they share one substrate — the per-step
//! (hold-plateau) aggregates every trial emits:
//!
//! - [`measure`]: **peak throughput** under a declared latency SLO, for the
//!   unpaced saturation suite.
//! - [`sustained_latency`]: **service latency at the ladder's fixed reference
//!   step**, judged against the curve of floor-scaled demand, for ramps whose
//!   whole-run latency aggregate would otherwise be dominated by queueing
//!   (see that function's docs).
//!
//! # The measurement
//!
//! A saturation workload is an unpaced (`pacing.mode = none`) closed-loop ramp:
//! each step ramps concurrency in, then *holds* it while the measurement is
//! taken. With no think time N virtual users are N requests in flight, so the
//! step's concurrency is the x-axis and `rps ~= N / service_time` until the
//! system runs out of capacity. Past that point rps flattens and latency climbs
//! linearly with N — the knee.
//!
//! # Why paced runs cannot produce this number
//!
//! Under `pacing.mode = drizzle-benchmark` each VU sleeps a mean 187.5 ms
//! between requests, so offered load is capped near `VUs / (think + service)`
//! regardless of how fast the target is. Every healthy target converges on the
//! same rps and a tenfold service-time difference barely moves it: the number
//! measures the sleep timer. That is why capacity lives in its own suite and the
//! paced suite keeps its own headline ("throughput at fixed load").
//!
//! # Reading the result
//!
//! A step *qualifies* when it met the SLO and stayed inside `limits.err`. The
//! peak is the **fastest** qualifying step, not the widest one: a closed-loop
//! curve dips once the pool saturates, so the last step to survive the SLO is
//! frequently slower than an earlier one that also survived it, and reporting
//! that under the name "peak throughput" would understate the target and point
//! at a worse operating point on both axes. Ties go to the lower concurrency.
//!
//! Whether the ramp found the ceiling is a separate question from where the
//! maximum landed, and `outcome` answers it: a ramp whose last step still
//! qualified never reached the knee, so its best throughput is reported as a
//! lower bound rather than a peak. When nothing qualifies there is no peak at
//! all. No case is padded with a substitute number.

use crate::code::{Code, Fail};
use crate::model::{
    CurveStepDoc, Limits, Outcome, PeakDoc, Point, SaturationDoc, SaturationSpec, Slo,
    StepLatencyDoc, SustainedLatencyDoc, SustainedOutcome, SustainedStepDoc,
};
use crate::stats::{avg, median};

/// How far above the last SLO-holding step the maximum must sit before the
/// curve counts as having turned over.
///
/// Throughput on a saturated closed-loop target wanders by a percent or two
/// between steps. Without a margin that wander alone would satisfy "the maximum
/// is not at the end" and turn every flat curve into a claimed peak, which is
/// the opposite of what this measurement is for.
const PLATEAU_MARGIN: f64 = 0.02;

/// Build the saturation artifact from one step series per trial.
///
/// Each `Point` is a whole hold plateau whose percentiles were computed from
/// that plateau's merged raw samples, so the per-step p99 is a real p99. Across
/// trials the reported value is the median, matching `summary.primary`.
///
/// # Errors
///
/// Fails when a trial produced no steps, when a step is missing its concurrency
/// tag or any percentile the curve reports, or when the trials disagree about
/// which steps ran. Each means the measurement is not the one the spec asked
/// for, and a patched-up curve would misrepresent where the knee is.
pub fn measure(
    target_id: &str,
    spec: &SaturationSpec,
    limits: &Limits,
    trials: &[&[Point]],
) -> Result<SaturationDoc, Fail> {
    let ladder = verified_ladder(target_id, trials)?;

    let curve = ladder
        .iter()
        .enumerate()
        .map(|(idx, &concurrency)| step(spec.slo, limits, concurrency, idx, trials))
        .collect::<Vec<_>>();

    // Peak throughput means the most throughput, not the most concurrency. A
    // closed-loop curve can dip after the pool saturates and then flatten, so
    // the highest-concurrency step that held the SLO is often slower than an
    // earlier one that also held it — reporting the former under the name "peak
    // throughput" would understate the target and point at a worse operating
    // point on both axes. Ties go to the lower concurrency: the same throughput
    // for fewer in-flight requests is strictly better.
    let best = curve.iter().filter(|step| step.qualifies()).max_by(|a, b| {
        a.rps
            .total_cmp(&b.rps)
            .then_with(|| b.concurrency.cmp(&a.concurrency))
    });

    // Whether the ramp found the ceiling is a question about *throughput*, not
    // about latency. A closed-loop target is saturated once adding in-flight
    // requests stops buying throughput — past that point the extra concurrency
    // becomes queueing and nothing else. So the ramp reached the ceiling if the
    // curve turned over: the best step is measurably faster than the last step
    // that still held the SLO.
    //
    // Keying this on "did some step breach the SLO" instead was wrong, and
    // measurably so. Every target here reaches maximum throughput at 4-16
    // concurrent requests and then holds it flat while p99 climbs from 0.2 ms to
    // 185 ms. A target fast enough to stay under the objective for the whole
    // ramp — rusqlite peaked at 62.8k rps at 16 and fell to 55.6k by 1024 — got
    // reported as "knee not reached" and lost its rank, when its maximum had in
    // fact been measured cleanly three steps in. Being fast is not a reason to
    // be excluded from a throughput ranking.
    //
    // The margin keeps run-to-run noise from manufacturing a turnover: a last
    // step a hair under the maximum is a plateau, and a plateau means the ramp
    // ended while throughput was still flat, which is exactly the case where the
    // honest answer is a lower bound.
    // The maximum has to be bracketed on both sides to count as measured: a rise
    // into it and a measurable fall out of it. A maximum sitting on the first
    // step is not bracketed — the curve may still have been climbing below the
    // ladder's floor — so it stays a lower bound rather than a peak the ramp did
    // not actually establish.
    let first_qualifying = curve.iter().position(|step| step.qualifies());
    let best_idx = best.and_then(|best| {
        curve
            .iter()
            .position(|step| std::ptr::eq(step, best as *const _))
    });
    let last_qualifying = curve.iter().rev().find(|step| step.qualifies());
    let turned_over = match (best, last_qualifying, best_idx, first_qualifying) {
        (Some(best), Some(last), Some(best_idx), Some(first_idx)) => {
            best_idx > first_idx && best.rps > last.rps * (1.0 + PLATEAU_MARGIN)
        }
        _ => false,
    };
    let ramp_ended_inside_slo = curve.last().is_some_and(|step| step.qualifies());

    let (outcome, peak, lower_bound_rps) = match best {
        None => (Outcome::SloNeverMet, None, None),
        // Known to reach at least this much while holding the SLO; throughput
        // was still flat or climbing when the ramp ran out, so the ceiling is
        // somewhere above it and was not measured.
        Some(step) if ramp_ended_inside_slo && !turned_over => {
            (Outcome::DidNotSaturate, None, Some(step.rps))
        }
        Some(step) => {
            let peak = PeakDoc {
                concurrency: step.concurrency,
                rps: step.rps,
                latency: step.latency,
                cpu: step.cpu,
                err: step.err,
            };
            (Outcome::Saturated, Some(peak), None)
        }
    };

    Ok(SaturationDoc {
        slo: spec.slo,
        outcome,
        peak,
        lower_bound_rps,
        curve,
    })
}

/// How far a step's throughput may fall short of its floor-scaled demand and
/// still count as sustained.
///
/// In a closed loop, an unsaturated target's per-VU throughput is constant in
/// N — `rps(N) = N / (think + latency)` with latency flat — so throughput
/// retention relative to the ladder's floor is 1.0 until the knee and
/// collapses past it. The recorded publish cohorts put numbers on both sides:
/// across every target and rung, retention on rungs below the knee never
/// measured under 0.92 (cross-trial medians; the wiggle on the noisiest host
/// was ≤3%), while the first rung past the knee never measured above 0.88 and
/// was usually ≤0.66, because the ladder's geometric spacing crosses the knee
/// in one rung. 0.10 sits in that gap with margin on both sides: more than 3x
/// the observed median-of-trials noise, and below every observed post-knee
/// retention.
///
/// What the tolerance admits is bounded too, and it is the instrument's
/// resolution limit: a rung can hide at most `tol/(1-tol) * (think + latency)`
/// of added delay — ~21 ms of mean queueing under the 187.5 ms pacing — before
/// its throughput shortfall trips this threshold. Queueing below that scale is
/// indistinguishable from service time in paced closed-loop throughput data;
/// the published retention discloses per rung how close to the limit the
/// reading ran.
pub const SUSTAINED_TOLERANCE: f64 = 0.10;

/// Read the target's service latency at the ladder's fixed reference step,
/// and judge every step of the curve against its floor-scaled demand.
///
/// A step's *offered* throughput is the floor step's measured rate scaled by
/// concurrency: in a closed loop, that is exactly what a target that kept its
/// floor latency would serve at that VU count. A step is **sustained** when it
/// served that demand within [`SUSTAINED_TOLERANCE`] and stayed inside the
/// error limit. This is deliberately not a latency SLO: any fixed latency
/// ceiling either denies slower targets a figure at the ladder's floor or,
/// widened until everyone passes, launders a saturated step's queueing as
/// service time. "Served what it was offered" needs no threshold on the
/// quantity being reported.
///
/// The published figure is read at the **second rung of the ladder** — the
/// lowest step with a non-vacuous sustained verdict — and not at the last
/// sustained rung. The last-sustained reading sits at the knee, the steepest
/// part of the latency curve, so which rung it lands on decides the figure;
/// replaying that rule over measured curves under ±3% throughput noise moved
/// the published p95 by 51-99% (one target swung 4.7 ms ↔ 73.2 ms on a
/// one-rung flap), and it compared different loads across targets — one row
/// read at 800 VUs against another at 100, which is not an ordering. The
/// fixed reference has no rung selection to perturb (measured figure movement
/// under the same noise: ~the latency noise itself) and reads every target at
/// the same offered load. The ladder places it at ≤35% utilization of the
/// slowest recorded target's ceiling, so its own retention rides far above
/// the tolerance threshold. Where a target *stopped* scaling stays in the
/// curve, one `sustained` flag per rung.
///
/// The floor itself sustains by identity (its retention is 1.0 against its own
/// rate), so it cannot vouch for itself; the reference step above it is what
/// corroborates that the floor sat below the knee. When the reference fails,
/// no figure is published: the floor's latency cannot be told apart from
/// queueing, and the honest fix is a ladder with lower rungs, not a floor
/// number that may already be queue time.
///
/// # Errors
///
/// Same contract as [`measure`], plus the ladder must have at least two steps
/// — a single rung has nothing to corroborate its floor against.
pub fn sustained_latency(
    target_id: &str,
    limits: &Limits,
    trials: &[&[Point]],
) -> Result<SustainedLatencyDoc, Fail> {
    let ladder = verified_ladder(target_id, trials)?;
    if ladder.len() < 2 {
        return Err(Fail::new(
            Code::AggregateFail,
            format!(
                "target {target_id} measured only {} step(s); the sustained-latency \
                 reading needs at least two — the floor is the reference other \
                 steps are judged against, so it cannot corroborate itself",
                ladder.len()
            ),
        ));
    }

    let stats: Vec<(u32, StepStats)> = ladder
        .iter()
        .enumerate()
        .map(|(idx, &concurrency)| (concurrency, step_stats(limits, idx, trials)))
        .collect();

    let (floor_vus, floor) = &stats[0];
    let floor_per_vu = floor.rps / f64::from(*floor_vus);

    let curve: Vec<SustainedStepDoc> = stats
        .iter()
        .map(|(concurrency, step)| {
            let offered_rps = floor_per_vu * f64::from(*concurrency);
            let retention = if offered_rps > 0.0 {
                step.rps / offered_rps
            } else {
                0.0
            };
            SustainedStepDoc {
                concurrency: *concurrency,
                rps: step.rps,
                offered_rps,
                retention,
                latency: step.latency,
                cpu: step.cpu,
                err: step.err,
                sustained: retention >= 1.0 - SUSTAINED_TOLERANCE && step.disqualified.is_none(),
                disqualified: step.disqualified.clone(),
            }
        })
        .collect();

    let (outcome, reference) = if curve[0].disqualified.is_some() {
        (SustainedOutcome::FloorDisqualified, None)
    } else if !curve[1].sustained {
        (SustainedOutcome::FloorAboveKnee, None)
    } else {
        (SustainedOutcome::Measured, Some(curve[1].clone()))
    };

    Ok(SustainedLatencyDoc {
        tolerance: SUSTAINED_TOLERANCE,
        outcome,
        reference,
        curve,
    })
}

/// The concurrency of every step, ascending, with every step verified complete
/// and the ladder verified identical across trials.
///
/// Trials are separate processes running the same spec; if their ladders differ,
/// one of them dropped or mis-tagged a step and medianing them would silently
/// compare different plateaus. Percentiles are required rather than defaulted:
/// filling a missing `p50` from `p95` would publish one number under another's
/// name, and a step that did not record its own percentiles was not measured.
fn verified_ladder(target_id: &str, trials: &[&[Point]]) -> Result<Vec<u32>, Fail> {
    let missing = |trial: usize, what: &str| {
        Fail::new(
            Code::AggregateFail,
            format!("target {target_id} trial {trial} emitted a saturation step without {what}"),
        )
    };

    let mut ladder: Option<Vec<u32>> = None;
    for (trial, steps) in trials.iter().enumerate() {
        if steps.is_empty() {
            return Err(Fail::new(
                Code::AggregateFail,
                format!(
                    "target {target_id} trial {trial} produced no saturation steps; \
                     a saturation workload must hold at each concurrency long enough \
                     to emit at least one hold bucket per step"
                ),
            ));
        }
        let found = steps
            .iter()
            .map(|point| {
                if point.latency.p50.is_none() {
                    return Err(missing(trial, "a p50 latency"));
                }
                if point.latency.p90.is_none() {
                    return Err(missing(trial, "a p90 latency"));
                }
                point
                    .vus
                    .ok_or_else(|| missing(trial, "a concurrency (vus) tag"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        match &ladder {
            None => ladder = Some(found),
            Some(first) if *first != found => {
                return Err(Fail::new(
                    Code::AggregateFail,
                    format!(
                        "target {target_id} trial {trial} ran concurrency ladder {found:?} \
                         but trial 0 ran {first:?}; trials must measure the same steps"
                    ),
                ));
            }
            Some(_) => {}
        }
    }

    ladder.ok_or_else(|| {
        Fail::new(
            Code::AggregateFail,
            format!("target {target_id} has no trials to build a saturation curve from"),
        )
    })
}

/// One step's cross-trial aggregates, before either measurement judges it.
struct StepStats {
    rps: f64,
    latency: StepLatencyDoc,
    err: f64,
    cpu: f64,
    disqualified: Option<String>,
}

/// Combine one step across trials: the median of the per-trial values, plus
/// the error-limit disqualification both measurements share.
fn step_stats(limits: &Limits, idx: usize, trials: &[&[Point]]) -> StepStats {
    let points: Vec<&Point> = trials.iter().map(|steps| &steps[idx]).collect();
    let per_trial =
        |value: fn(&Point) -> f64| median(&points.iter().copied().map(value).collect::<Vec<_>>());

    // `expect` is discharged by `verified_ladder`, which rejects any step point
    // missing a percentile. Substituting a neighbouring percentile would publish
    // one number under another's name.
    let latency = StepLatencyDoc {
        p50: per_trial(|point| point.latency.p50.expect("p50 verified present")),
        p90: per_trial(|point| point.latency.p90.expect("p90 verified present")),
        p95: per_trial(|point| point.latency.p95),
        p99: per_trial(|point| point.latency.p99),
    };
    let err = per_trial(|point| point.err);

    StepStats {
        rps: per_trial(|point| point.rps),
        latency,
        err,
        cpu: per_trial(|point| avg(&point.cpu)),
        disqualified: (err > limits.err).then(|| {
            format!(
                "error rate {:.2}% exceeds limit {:.2}%",
                err * 100.0,
                limits.err * 100.0
            )
        }),
    }
}

/// Combine one step across trials and judge it against the SLO and error limit.
fn step(
    slo: Slo,
    limits: &Limits,
    concurrency: u32,
    idx: usize,
    trials: &[&[Point]],
) -> CurveStepDoc {
    let stats = step_stats(limits, idx, trials);
    CurveStepDoc {
        concurrency,
        rps: stats.rps,
        latency: stats.latency,
        err: stats.err,
        cpu: stats.cpu,
        slo_met: slo.metric.of(&stats.latency) <= slo.ms,
        disqualified: stats.disqualified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Latency, Phase, Slo, SloMetric};

    fn spec(ms: f64) -> SaturationSpec {
        SaturationSpec {
            slo: Slo {
                metric: SloMetric::P99,
                ms,
            },
        }
    }

    fn limits(err: f64) -> Limits {
        Limits {
            err,
            p95: None,
            cpu_mean_peak: None,
        }
    }

    /// One measured step: concurrency, throughput, p99, and error rate.
    fn point(vus: u32, rps: f64, p99: f64, err: f64) -> Point {
        Point {
            time: "2026-01-01T00:00:00Z".to_string(),
            rps,
            err,
            latency: Latency {
                avg: p99 / 4.0,
                p50: Some(p99 / 4.0),
                p90: Some(p99 / 2.0),
                p95: p99 * 0.8,
                p99,
                p999: Some(p99 * 1.2),
            },
            cpu: vec![50.0, 50.0],
            mem_mb: None,
            trial: Some(0),
            stage: None,
            phase: Some(Phase::Hold),
            vus: Some(vus),
            probe: false,
            requests: Some(rps as u64),
            queries: Vec::new(),
        }
    }

    fn measured(steps: &[Point], slo_ms: f64, err_limit: f64) -> SaturationDoc {
        measure("t", &spec(slo_ms), &limits(err_limit), &[steps]).expect("measure")
    }

    /// The case that distinguishes "most throughput" from "most concurrency".
    /// Throughput peaks at 16 VUs and sags afterwards while still holding the
    /// SLO up to 64; the highest *qualifying concurrency* is 64, but the peak
    /// *throughput* is at 16. Reporting 2 100 under the label "peak throughput"
    /// when 3 100 was measured inside the same SLO would understate the target
    /// and point at a worse operating point on both axes.
    #[test]
    fn peak_is_the_fastest_qualifying_step_not_the_widest() {
        let steps = [
            point(8, 2_400.0, 4.0, 0.0),
            point(16, 3_100.0, 9.0, 0.0),
            point(32, 2_600.0, 18.0, 0.0),
            point(64, 2_100.0, 40.0, 0.0),
            point(128, 1_900.0, 120.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::Saturated);
        let peak = doc.peak.expect("peak");
        assert_eq!(peak.rps, 3_100.0);
        assert_eq!(peak.concurrency, 16);
        assert_eq!(peak.latency.p99, 9.0);
        // 64 held the SLO and is the widest qualifying step; it is not the peak.
        assert!(doc.curve[3].qualifies());
    }

    /// Same throughput for fewer in-flight requests is strictly better.
    #[test]
    fn a_throughput_tie_breaks_toward_the_lower_concurrency() {
        let steps = [
            point(8, 1_000.0, 4.0, 0.0),
            point(16, 2_500.0, 9.0, 0.0),
            point(32, 2_500.0, 19.0, 0.0),
            point(64, 2_500.0, 120.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        let peak = doc.peak.expect("peak");
        assert_eq!(peak.rps, 2_500.0);
        assert_eq!(peak.concurrency, 16);
    }

    /// A curve that rises into a maximum and measurably falls out of it has
    /// found the ceiling, whether or not any step breached the objective.
    ///
    /// Saturation is a property of throughput: once more in-flight requests stop
    /// buying throughput, the target is at its limit and the extra concurrency is
    /// only queueing. Requiring an SLO breach on top of that punished targets for
    /// being fast — a real run had a target peak at 62.8k and fall to 55.6k while
    /// never crossing a 25 ms p99, and it was reported as "knee not reached" and
    /// dropped from the ranking.
    #[test]
    fn a_curve_that_turns_over_inside_the_slo_reports_a_peak() {
        let steps = [
            point(8, 1_000.0, 2.0, 0.0),
            point(16, 3_000.0, 4.0, 0.0),
            point(32, 2_400.0, 9.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::Saturated);
        let peak = doc.peak.as_ref().expect("peak");
        assert_eq!(peak.rps, 3_000.0);
        assert_eq!(peak.concurrency, 16);
        assert!(doc.lower_bound_rps.is_none());
    }

    /// A curve that flattens has not turned over: the last step is within noise
    /// of the maximum, so the ramp ended while throughput was still level and the
    /// ceiling is above it. Without the margin, ordinary run-to-run wander would
    /// manufacture a peak out of every flat curve.
    #[test]
    fn a_plateau_inside_the_slo_is_a_lower_bound_not_a_peak() {
        let steps = [
            point(8, 1_000.0, 2.0, 0.0),
            point(16, 3_000.0, 4.0, 0.0),
            point(32, 2_985.0, 9.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::DidNotSaturate);
        assert!(doc.peak.is_none());
        assert_eq!(doc.lower_bound_rps, Some(3_000.0));
    }

    /// On a monotone ramp the fastest qualifying step *is* the widest one, so
    /// the two readings coincide. This is the shape the method assumes.
    #[test]
    fn a_monotone_ramp_peaks_at_its_last_qualifying_step() {
        let steps = [
            point(8, 1_000.0, 9.0, 0.0),
            point(16, 1_900.0, 18.0, 0.0),
            point(32, 2_000.0, 45.0, 0.0),
            point(64, 2_010.0, 120.0, 0.0),
            point(128, 2_005.0, 260.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::Saturated);
        assert!(doc.lower_bound_rps.is_none());
        let peak = doc.peak.expect("saturated runs carry a peak");
        assert_eq!(peak.concurrency, 32);
        assert_eq!(peak.rps, 2_000.0);
        assert_eq!(peak.latency.p99, 45.0);
        // The whole ramp is reported, breaches included.
        assert_eq!(doc.curve.len(), 5);
        assert_eq!(
            doc.curve.iter().map(|s| s.slo_met).collect::<Vec<_>>(),
            vec![true, true, true, false, false]
        );
    }

    /// The peak is lifted out of the curve rather than recomputed, so its
    /// concurrency is always one of the plotted steps. Consumers mark the peak
    /// on the curve and deliberately withhold the marker rather than snapping it
    /// to a neighbour, which would make any drift here visible.
    #[test]
    fn the_peak_is_always_one_of_the_plotted_steps() {
        let steps = [
            point(8, 1_000.0, 9.0, 0.0),
            point(16, 1_900.0, 18.0, 0.0),
            point(32, 2_000.0, 45.0, 0.0),
            point(64, 2_010.0, 120.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);
        let peak = doc.peak.expect("peak");

        let plotted = doc
            .curve
            .iter()
            .find(|step| step.concurrency == peak.concurrency)
            .expect("peak concurrency must appear in the curve");
        assert_eq!(plotted.rps, peak.rps);
        assert_eq!(plotted.latency.p99, peak.latency.p99);
        assert_eq!(plotted.err, peak.err);
        assert_eq!(plotted.cpu, peak.cpu);
    }

    #[test]
    fn no_qualifying_step_reports_no_peak_at_all() {
        let steps = [
            point(8, 1_000.0, 80.0, 0.0),
            point(16, 1_200.0, 190.0, 0.0),
            point(32, 1_150.0, 400.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::SloNeverMet);
        // The smallest step must never be promoted to a peak it did not earn.
        assert!(doc.peak.is_none());
        assert!(doc.lower_bound_rps.is_none());
        assert_eq!(doc.curve.len(), 3);
        assert!(doc.curve.iter().all(|step| !step.slo_met));
    }

    #[test]
    fn a_ramp_that_never_breaks_reports_a_lower_bound_not_a_peak() {
        let steps = [
            point(8, 1_000.0, 4.0, 0.0),
            point(16, 2_000.0, 8.0, 0.0),
            point(32, 4_000.0, 16.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::DidNotSaturate);
        assert!(doc.peak.is_none());
        assert_eq!(doc.lower_bound_rps, Some(4_000.0));
    }

    /// A maximum on the very first step is not bracketed: the curve may still
    /// have been climbing below the ladder's floor, so the ramp started at or
    /// above the knee rather than finding it. That stays a lower bound.
    #[test]
    fn an_early_maximum_does_not_by_itself_mean_saturated() {
        let steps = [
            point(8, 5_000.0, 2.0, 0.0),
            point(16, 3_000.0, 4.0, 0.0),
            point(32, 2_800.0, 9.0, 0.0),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::DidNotSaturate);
        assert_eq!(doc.lower_bound_rps, Some(5_000.0));
    }

    #[test]
    fn an_over_error_step_is_disqualified_and_cannot_be_the_peak() {
        let steps = [
            point(8, 1_000.0, 4.0, 0.0),
            point(16, 2_000.0, 8.0, 0.0),
            // Fast, well inside the SLO, but failing 3.2% of requests.
            point(32, 3_800.0, 12.0, 0.032),
            point(64, 3_900.0, 90.0, 0.05),
        ];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::Saturated);
        let peak = doc.peak.expect("peak");
        assert_eq!(peak.concurrency, 16, "a disqualified step cannot be peak");
        assert_eq!(
            doc.curve[2].disqualified.as_deref(),
            Some("error rate 3.20% exceeds limit 1.00%")
        );
        // Disqualification is recorded, never silently skipped.
        assert!(doc.curve[2].slo_met, "it met the SLO; it failed on errors");
        assert!(doc.curve[0].disqualified.is_none());
    }

    #[test]
    fn every_step_disqualified_by_errors_yields_no_peak() {
        let steps = [point(8, 1_000.0, 4.0, 0.5), point(16, 900.0, 8.0, 0.6)];
        let doc = measured(&steps, 50.0, 0.01);

        assert_eq!(doc.outcome, Outcome::SloNeverMet);
        assert!(doc.peak.is_none());
        assert!(doc.curve.iter().all(|step| step.disqualified.is_some()));
    }

    #[test]
    fn steps_are_medianed_across_trials() {
        let a = [point(8, 1_000.0, 10.0, 0.0), point(16, 1_500.0, 90.0, 0.0)];
        let b = [point(8, 1_400.0, 12.0, 0.0), point(16, 1_600.0, 95.0, 0.0)];
        let c = [point(8, 1_200.0, 11.0, 0.0), point(16, 1_550.0, 92.0, 0.0)];

        let doc = measure(
            "t",
            &spec(50.0),
            &limits(0.01),
            &[a.as_slice(), b.as_slice(), c.as_slice()],
        )
        .expect("measure");

        assert_eq!(doc.curve[0].rps, 1_200.0);
        assert_eq!(doc.curve[0].latency.p99, 11.0);
        assert_eq!(doc.outcome, Outcome::Saturated);
        assert_eq!(doc.peak.expect("peak").rps, 1_200.0);
    }

    #[test]
    fn trials_that_ran_different_ladders_are_refused() {
        let a = [point(8, 1_000.0, 10.0, 0.0), point(16, 1_500.0, 20.0, 0.0)];
        let b = [point(8, 1_000.0, 10.0, 0.0), point(32, 1_500.0, 20.0, 0.0)];

        let err = measure(
            "drizzle-rs-sqlite",
            &spec(50.0),
            &limits(0.01),
            &[a.as_slice(), b.as_slice()],
        )
        .expect_err("mismatched ladders must fail");
        assert!(
            err.msg.contains("must measure the same steps"),
            "{}",
            err.msg
        );
    }

    #[test]
    fn a_trial_with_no_steps_is_refused() {
        let err = measure("t", &spec(50.0), &limits(0.01), &[&[]])
            .expect_err("an empty ladder must fail");
        assert!(err.msg.contains("no saturation steps"), "{}", err.msg);
    }

    /// A missing percentile is a missing measurement. Filling it from a
    /// neighbouring percentile would publish one number under another's name.
    #[test]
    fn a_step_missing_a_percentile_is_refused() {
        let mut steps = [point(8, 1_000.0, 10.0, 0.0), point(16, 1_500.0, 20.0, 0.0)];
        steps[1].latency.p50 = None;
        let err = measure("t", &spec(50.0), &limits(0.01), &[steps.as_slice()])
            .expect_err("missing p50 must fail");
        assert!(err.msg.contains("without a p50 latency"), "{}", err.msg);

        let mut steps = [point(8, 1_000.0, 10.0, 0.0), point(16, 1_500.0, 20.0, 0.0)];
        steps[0].vus = None;
        let err = measure("t", &spec(50.0), &limits(0.01), &[steps.as_slice()])
            .expect_err("missing vus must fail");
        assert!(err.msg.contains("without a concurrency"), "{}", err.msg);
    }

    fn sustained(steps: &[Point]) -> SustainedLatencyDoc {
        sustained_latency("t", &limits(0.01), &[steps]).expect("sustained latency")
    }

    /// The figure is read at the ladder's second rung — the lowest step with a
    /// non-vacuous sustained verdict — not at the knee. The knee still shows
    /// in the curve's per-rung flags; it just cannot decide the headline.
    #[test]
    fn latency_is_read_at_the_fixed_reference_step() {
        let steps = [
            point(50, 260.0, 10.0, 0.0),
            point(100, 512.0, 12.0, 0.0), // retention 0.98 — the reference
            point(200, 940.0, 40.0, 0.0), // retention 0.90 — still sustained
            point(400, 1_300.0, 500.0, 0.0), // retention 0.63 — the knee
        ];
        let doc = sustained(&steps);

        assert_eq!(doc.outcome, SustainedOutcome::Measured);
        let reference = doc.reference.expect("reference");
        assert_eq!(reference.concurrency, 100);
        assert_eq!(reference.latency.p95, 12.0 * 0.8); // p95 = 0.8 * p99 in `point`
        // The criterion's inputs are published on every rung.
        assert_eq!(doc.tolerance, SUSTAINED_TOLERANCE);
        assert_eq!(doc.curve[2].offered_rps, 1_040.0); // 260/50 * 200
        assert!((doc.curve[2].retention - 940.0 / 1_040.0).abs() < 1e-12);
        assert_eq!(
            doc.curve.iter().map(|s| s.sustained).collect::<Vec<_>>(),
            vec![true, true, true, false]
        );
    }

    /// The defect that retired the last-sustained-rung reading: the knee is
    /// the steepest part of the latency curve, so a one-rung flap there moved
    /// the published figure by 51-99% in replays over measured curves. Moving
    /// the knee must not move the reference figure.
    #[test]
    fn the_reference_does_not_move_with_the_knee() {
        let knee_at_200 = [
            point(50, 260.0, 10.0, 0.0),
            point(100, 512.0, 12.0, 0.0),
            point(200, 830.0, 150.0, 0.0), // collapsed
            point(400, 900.0, 500.0, 0.0),
        ];
        let knee_at_400 = [
            point(50, 260.0, 10.0, 0.0),
            point(100, 512.0, 12.0, 0.0),
            point(200, 990.0, 40.0, 0.0),    // sustained this time
            point(400, 1_050.0, 500.0, 0.0), // collapsed
        ];

        let early = sustained(&knee_at_200).reference.expect("reference");
        let late = sustained(&knee_at_400).reference.expect("reference");

        assert_eq!(early.concurrency, 100);
        assert_eq!(late.concurrency, 100);
        assert_eq!(early.latency.p95, late.latency.p95);
    }

    /// A ramp the target never stopped scaling on reads at the same reference
    /// step as everyone else: the headline compares like loads, and the
    /// curve's flags say the target was still scaling at the top.
    #[test]
    fn sustaining_the_whole_ramp_still_reads_at_the_reference() {
        let steps = [
            point(50, 265.0, 1.0, 0.0),
            point(100, 530.0, 1.0, 0.0),
            point(200, 1_060.0, 1.0, 0.0),
        ];
        let doc = sustained(&steps);

        assert_eq!(doc.outcome, SustainedOutcome::Measured);
        assert_eq!(doc.reference.expect("reference").concurrency, 100);
        assert!(doc.curve.iter().all(|s| s.sustained));
    }

    /// The floor's retention is 1.0 against its own rate, so it cannot vouch
    /// for itself; the reference rung above it is the corroboration. When it
    /// fails, the floor's latency cannot be told apart from queueing and no
    /// figure is published — the fix is a ladder with lower rungs, and
    /// publishing the floor anyway would launder exactly the number this
    /// measurement exists to stop.
    #[test]
    fn a_failed_reference_reports_no_figure() {
        let steps = [
            point(50, 240.0, 60.0, 0.0),
            point(100, 250.0, 250.0, 0.0), // flat: ceiling at or below the floor
            point(200, 255.0, 600.0, 0.0),
        ];
        let doc = sustained(&steps);

        assert_eq!(doc.outcome, SustainedOutcome::FloorAboveKnee);
        assert!(doc.reference.is_none());
        assert_eq!(doc.curve.len(), 3);
        // The floor's identity-retention is visible in the artifact, so a
        // reader can see why it could not vouch for itself.
        assert_eq!(doc.curve[0].retention, 1.0);
        assert!(!doc.curve[1].sustained);
    }

    /// Fast answers on a step that sheds load through errors are not
    /// "sustained": survivor latency is biased, so the disqualification marks
    /// the rung exactly like a throughput collapse — visible in the curve,
    /// while the reference below it is untouched.
    #[test]
    fn an_error_disqualified_rung_is_marked_unsustained() {
        let steps = [
            point(50, 260.0, 10.0, 0.0),
            point(100, 515.0, 11.0, 0.0),
            point(200, 1_030.0, 12.0, 0.05), // retention 0.99 but failing 5%
            point(400, 2_060.0, 13.0, 0.0),
        ];
        let doc = sustained(&steps);

        assert_eq!(doc.outcome, SustainedOutcome::Measured);
        assert_eq!(doc.reference.expect("reference").concurrency, 100);
        assert!(doc.curve[2].disqualified.is_some());
        assert!(!doc.curve[2].sustained);
    }

    /// An erroring reference rung is inadmissible as scaling evidence — its
    /// throughput includes shed load — so the floor is left uncorroborated,
    /// the same as a retention collapse there.
    #[test]
    fn an_erroring_reference_leaves_the_floor_uncorroborated() {
        let steps = [
            point(50, 260.0, 10.0, 0.0),
            point(100, 515.0, 11.0, 0.05),
            point(200, 1_030.0, 12.0, 0.0),
        ];
        let doc = sustained(&steps);

        assert_eq!(doc.outcome, SustainedOutcome::FloorAboveKnee);
        assert!(doc.reference.is_none());
    }

    /// An erroring floor has no honest latency at all: the yardstick itself is
    /// survivorship-biased, so nothing judged against it can be trusted either.
    #[test]
    fn an_erroring_floor_disqualifies_the_whole_reading() {
        let steps = [
            point(50, 260.0, 10.0, 0.5),
            point(100, 520.0, 11.0, 0.0),
            point(200, 1_040.0, 12.0, 0.0),
        ];
        let doc = sustained(&steps);

        assert_eq!(doc.outcome, SustainedOutcome::FloorDisqualified);
        assert!(doc.reference.is_none());
        assert!(doc.curve[0].disqualified.is_some());
    }

    /// A single rung has nothing to corroborate its floor against, and the
    /// spec validation enforces the same bound up front.
    #[test]
    fn a_single_rung_ladder_is_refused() {
        let steps = [point(50, 260.0, 10.0, 0.0)];
        let err = sustained_latency("t", &limits(0.01), &[steps.as_slice()])
            .expect_err("one rung must fail");
        assert!(err.msg.contains("at least two"), "{}", err.msg);
    }

    /// Like the saturation peak, the reference is lifted out of the curve and
    /// medianed across trials, never recomputed.
    #[test]
    fn sustained_steps_are_medianed_across_trials() {
        let a = [
            point(50, 250.0, 10.0, 0.0),
            point(100, 495.0, 12.0, 0.0),
            point(200, 700.0, 300.0, 0.0),
        ];
        let b = [
            point(50, 270.0, 14.0, 0.0),
            point(100, 515.0, 16.0, 0.0),
            point(200, 740.0, 320.0, 0.0),
        ];
        let c = [
            point(50, 260.0, 12.0, 0.0),
            point(100, 505.0, 14.0, 0.0),
            point(200, 720.0, 310.0, 0.0),
        ];

        let doc = sustained_latency(
            "t",
            &limits(0.01),
            &[a.as_slice(), b.as_slice(), c.as_slice()],
        )
        .expect("sustained latency");

        assert_eq!(doc.outcome, SustainedOutcome::Measured);
        let reference = doc.reference.expect("reference");
        assert_eq!(reference.concurrency, 100);
        assert_eq!(reference.rps, 505.0);
        assert_eq!(reference.latency.p99, 14.0);
        // Offered is scaled from the medianed floor, 260 rps at 50 VUs.
        assert_eq!(reference.offered_rps, 520.0);
    }

    /// Both readings share `verified_ladder`, so a torn ladder is refused here
    /// the same way it is for the capacity measurement.
    #[test]
    fn sustained_refuses_trials_that_ran_different_ladders() {
        let a = [point(50, 260.0, 10.0, 0.0), point(100, 500.0, 20.0, 0.0)];
        let b = [point(50, 260.0, 10.0, 0.0), point(200, 500.0, 20.0, 0.0)];

        let err = sustained_latency("t", &limits(0.01), &[a.as_slice(), b.as_slice()])
            .expect_err("mismatched ladders must fail");
        assert!(
            err.msg.contains("must measure the same steps"),
            "{}",
            err.msg
        );
    }

    #[test]
    fn the_declared_metric_decides_the_slo() {
        // p99 = 60 breaches a 50 ms p99 SLO, but p95 = 48 passes a 50 ms p95 one.
        let steps = [point(8, 1_000.0, 60.0, 0.0), point(16, 1_100.0, 300.0, 0.0)];

        let on_p99 = measured(&steps, 50.0, 0.01);
        assert_eq!(on_p99.outcome, Outcome::SloNeverMet);

        let on_p95 = measure(
            "t",
            &SaturationSpec {
                slo: Slo {
                    metric: SloMetric::P95,
                    ms: 50.0,
                },
            },
            &limits(0.01),
            &[steps.as_slice()],
        )
        .expect("measure");
        assert_eq!(on_p95.outcome, Outcome::Saturated);
        assert_eq!(on_p95.peak.expect("peak").concurrency, 8);
    }
}
