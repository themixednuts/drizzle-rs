//! Peak throughput under a declared latency SLO.
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
    CurveStepDoc, Limits, Outcome, PeakDoc, Point, SaturationDoc, SaturationSpec, StepLatencyDoc,
};
use crate::stats::{avg, median};

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
        .map(|(idx, &concurrency)| step(spec, limits, concurrency, idx, trials))
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

    // Whether the ramp ever found the ceiling is a separate question from where
    // the best step was: a ramp whose last step still held the SLO never reached
    // the knee, however early the maximum happened to land.
    let ramp_ended_inside_slo = curve.last().is_some_and(|step| step.qualifies());

    let (outcome, peak, lower_bound_rps) = match best {
        None => (Outcome::SloNeverMet, None, None),
        // Known to reach at least this much while holding the SLO; the ceiling
        // is somewhere above the ramp and was not measured.
        Some(step) if ramp_ended_inside_slo => (Outcome::DidNotSaturate, None, Some(step.rps)),
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

/// Combine one step across trials and judge it against the SLO and error limit.
fn step(
    spec: &SaturationSpec,
    limits: &Limits,
    concurrency: u32,
    idx: usize,
    trials: &[&[Point]],
) -> CurveStepDoc {
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

    CurveStepDoc {
        concurrency,
        rps: per_trial(|point| point.rps),
        latency,
        err,
        cpu: per_trial(|point| avg(&point.cpu)),
        slo_met: spec.slo.metric.of(&latency) <= spec.slo.ms,
        disqualified: (err > limits.err).then(|| {
            format!(
                "error rate {:.2}% exceeds limit {:.2}%",
                err * 100.0,
                limits.err * 100.0
            )
        }),
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

    /// A ramp that never breaches reports its best qualifying throughput as a
    /// lower bound — the maximum is still the maximum, the ceiling is what was
    /// not found.
    #[test]
    fn a_lower_bound_is_the_best_qualifying_throughput() {
        let steps = [
            point(8, 1_000.0, 2.0, 0.0),
            point(16, 3_000.0, 4.0, 0.0),
            point(32, 2_400.0, 9.0, 0.0),
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

    /// The ramp reaching its end inside the SLO is what makes an outcome
    /// `did_not_saturate` — not where the maximum happened to land.
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
