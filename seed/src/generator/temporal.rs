use super::{Generator, RngCore, SeedValue};
use rand::Rng;

/// Days from 1970-01-01 to the given civil date (proleptic Gregorian).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Generates random dates from 2000-01-01 to 2030-12-28 (days 1 to 28) as
/// `YYYY-MM-DD` text, or as Unix milliseconds at midnight UTC when the column
/// type contains `INT`.
pub struct DateGen;

impl Generator for DateGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, sql_type: &str) -> SeedValue {
        let year = rng.random_range(2000u16..=2030);
        let month = rng.random_range(1u8..=12);
        let day = rng.random_range(1u8..=28); // safe for all months
        let upper = sql_type.to_uppercase();
        if upper.contains("INT") || upper.contains("BIGINT") {
            // SQLite timestamp_ms mode: milliseconds since the Unix epoch.
            let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
            SeedValue::Integer(days * 86_400_000)
        } else {
            SeedValue::Text(format!("{year:04}-{month:02}-{day:02}"))
        }
    }
    fn name(&self) -> &'static str {
        "Date"
    }
}

/// Generates random timestamps from 2000 to 2030 as `YYYY-MM-DD HH:MM:SS`
/// text, or as Unix milliseconds (UTC) when the column type contains `INT`.
pub struct TimestampGen;

impl Generator for TimestampGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, sql_type: &str) -> SeedValue {
        let year = rng.random_range(2000u16..=2030);
        let month = rng.random_range(1u8..=12);
        let day = rng.random_range(1u8..=28);
        let hour = rng.random_range(0u8..=23);
        let minute = rng.random_range(0u8..=59);
        let second = rng.random_range(0u8..=59);
        let upper = sql_type.to_uppercase();
        if upper.contains("INT") || upper.contains("BIGINT") {
            // SQLite timestamp_ms mode: milliseconds since the Unix epoch.
            let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
            let secs =
                days * 86_400 + i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
            SeedValue::Integer(secs * 1000)
        } else {
            SeedValue::Text(format!(
                "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}"
            ))
        }
    }
    fn name(&self) -> &'static str {
        "Timestamp"
    }
}

/// Generates random times in HH:MM:SS format.
pub struct TimeGen;

impl Generator for TimeGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let hour = rng.random_range(0u8..=23);
        let minute = rng.random_range(0u8..=59);
        let second = rng.random_range(0u8..=59);
        SeedValue::Text(format!("{hour:02}:{minute:02}:{second:02}"))
    }
    fn name(&self) -> &'static str {
        "Time"
    }
}

/// Generates random times with timezone offsets in HH:MM:SS+HH format.
pub struct TimeTzGen;

impl Generator for TimeTzGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        use rand::Rng;
        let hour = rng.random_range(0u8..=23);
        let minute = rng.random_range(0u8..=59);
        let second = rng.random_range(0u8..=59);
        let offset = rng.random_range(-12i8..=14);
        SeedValue::Text(format!("{hour:02}:{minute:02}:{second:02}{offset:+03}"))
    }
    fn name(&self) -> &'static str {
        "TimeTz"
    }
}

/// Generates simple `PostgreSQL` interval strings.
pub struct IntervalGen;

impl Generator for IntervalGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        use rand::Rng;
        let amount = rng.random_range(1u16..=72);
        SeedValue::Text(format!("{amount} hours"))
    }
    fn name(&self) -> &'static str {
        "Interval"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn date_format_and_valid_range() {
        let g = DateGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            match g.generate(&mut rng, 0, "TEXT") {
                SeedValue::Text(s) => {
                    assert_eq!(s.len(), 10, "date wrong length: {}", s);
                    assert_eq!(&s[4..5], "-");
                    assert_eq!(&s[7..8], "-");
                    let year: u16 = s[0..4].parse().unwrap();
                    let month: u8 = s[5..7].parse().unwrap();
                    let day: u8 = s[8..10].parse().unwrap();
                    assert!((2000..=2030).contains(&year), "year out of range: {}", year);
                    assert!((1..=12).contains(&month), "month out of range: {}", month);
                    assert!((1..=28).contains(&day), "day out of range: {}", day);
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn timestamp_format_and_valid_parts() {
        let g = TimestampGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            match g.generate(&mut rng, 0, "TEXT") {
                SeedValue::Text(s) => {
                    assert_eq!(s.len(), 19, "timestamp wrong length: {}", s);
                    assert_eq!(&s[10..11], " ");
                    assert_eq!(&s[13..14], ":");
                    assert_eq!(&s[16..17], ":");
                    let hour: u8 = s[11..13].parse().unwrap();
                    let minute: u8 = s[14..16].parse().unwrap();
                    let second: u8 = s[17..19].parse().unwrap();
                    assert!(hour <= 23, "hour out of range: {}", hour);
                    assert!(minute <= 59, "minute out of range: {}", minute);
                    assert!(second <= 59, "second out of range: {}", second);
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn date_integer_for_int_columns() {
        let g = DateGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            match g.generate(&mut rng, 0, "INTEGER") {
                SeedValue::Integer(ms) => {
                    assert!(ms > 0, "date ms should be positive: {ms}");
                }
                other => panic!("expected Integer for INTEGER column, got: {:?}", other),
            }
        }
    }

    #[test]
    fn timestamp_integer_for_bigint_columns() {
        let g = TimestampGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            match g.generate(&mut rng, 0, "BIGINT") {
                SeedValue::Integer(ms) => {
                    assert!(ms > 0, "timestamp ms should be positive: {ms}");
                }
                other => panic!("expected Integer for BIGINT column, got: {:?}", other),
            }
        }
    }

    #[test]
    fn time_format_and_valid_parts() {
        let g = TimeGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            match g.generate(&mut rng, 0, "TEXT") {
                SeedValue::Text(s) => {
                    assert_eq!(s.len(), 8, "time wrong length: {}", s);
                    assert_eq!(&s[2..3], ":");
                    assert_eq!(&s[5..6], ":");
                    let hour: u8 = s[0..2].parse().unwrap();
                    let minute: u8 = s[3..5].parse().unwrap();
                    let second: u8 = s[6..8].parse().unwrap();
                    assert!(hour <= 23, "hour out of range: {}", hour);
                    assert!(minute <= 59, "minute out of range: {}", minute);
                    assert!(second <= 59, "second out of range: {}", second);
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn generators_are_deterministic() {
        for seed in [0u64, 42, 999, u64::MAX] {
            let mut rng1 = StdRng::seed_from_u64(seed);
            let mut rng2 = StdRng::seed_from_u64(seed);
            assert_eq!(
                DateGen.generate(&mut rng1, 0, "TEXT"),
                DateGen.generate(&mut rng2, 0, "TEXT")
            );
            let mut rng1 = StdRng::seed_from_u64(seed);
            let mut rng2 = StdRng::seed_from_u64(seed);
            assert_eq!(
                TimestampGen.generate(&mut rng1, 5, "TEXT"),
                TimestampGen.generate(&mut rng2, 5, "TEXT")
            );
            let mut rng1 = StdRng::seed_from_u64(seed);
            let mut rng2 = StdRng::seed_from_u64(seed);
            assert_eq!(
                TimeGen.generate(&mut rng1, 10, "TEXT"),
                TimeGen.generate(&mut rng2, 10, "TEXT")
            );
        }
    }

    #[test]
    fn timetz_format_contains_offset() {
        let g = TimeTzGen;
        let mut rng = StdRng::seed_from_u64(42);
        match g.generate(&mut rng, 0, "TEXT") {
            SeedValue::Text(s) => {
                assert!(s.len() >= 11, "timetz too short: {s}");
                assert!(
                    s.contains('+') || s.contains('-'),
                    "timetz missing offset: {s}"
                );
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn interval_looks_like_interval_literal() {
        let g = IntervalGen;
        let mut rng = StdRng::seed_from_u64(42);
        match g.generate(&mut rng, 0, "TEXT") {
            SeedValue::Text(s) => {
                assert!(s.ends_with(" hours"), "interval format mismatch: {s}");
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn days_from_civil_matches_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(2024, 2, 29), 19_782);
        assert_eq!(days_from_civil(2030, 12, 28), 22_276);
    }
}
