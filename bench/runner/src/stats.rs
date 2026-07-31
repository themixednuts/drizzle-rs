//! Summary statistics shared by the load generator and the aggregator.
//!
//! `f64::total_cmp` is used throughout rather than `partial_cmp().unwrap()`:
//! latency samples come from timers and a stray NaN must not panic a run that
//! took minutes to produce.

pub fn avg(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

pub fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut items = values.to_vec();
    items.sort_by(f64::total_cmp);
    median_sorted(&items)
}

pub fn median_sorted(items: &[f64]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    let mid = items.len() / 2;
    if items.len() % 2 == 1 {
        items[mid]
    } else {
        (items[mid - 1] + items[mid]) / 2.0
    }
}

/// Nearest-rank percentile over an already-sorted slice.
///
/// Callers that need several percentiles sort once and call this repeatedly;
/// re-sorting per percentile is measurable at trial-aggregate sample counts.
pub fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

pub fn peak(values: &[f64]) -> f64 {
    values.iter().fold(0.0_f64, |acc, val| acc.max(*val))
}

pub fn min(values: &[f64]) -> f64 {
    values.iter().copied().reduce(f64::min).unwrap_or(0.0)
}

pub fn max(values: &[f64]) -> f64 {
    values.iter().copied().reduce(f64::max).unwrap_or(0.0)
}

/// Bessel-corrected sample variance. Zero for fewer than two samples.
pub fn sample_variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = avg(values);
    values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() - 1) as f64
}

#[cfg(test)]
mod tests {
    use super::{median, percentile_sorted, sample_variance};

    #[test]
    fn median_handles_even_and_odd_lengths() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), 2.5);
        assert_eq!(median(&[]), 0.0);
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        let sorted: Vec<f64> = (0..=100).map(f64::from).collect();
        assert_eq!(percentile_sorted(&sorted, 0.5), 50.0);
        assert_eq!(percentile_sorted(&sorted, 0.95), 95.0);
        assert_eq!(percentile_sorted(&sorted, 1.0), 100.0);
        assert_eq!(percentile_sorted(&[], 0.95), 0.0);
    }

    #[test]
    fn variance_needs_two_samples() {
        assert_eq!(sample_variance(&[5.0]), 0.0);
        assert_eq!(sample_variance(&[100.0, 200.0, 300.0]), 10_000.0);
    }
}
