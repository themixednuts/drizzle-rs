//! Text encoding for `chrono::Duration` and `time::Duration` values.
//!
//! Durations are stored as `[-]<seconds>[.<fraction>]s`, for example `90s`,
//! `1.5s` or `-0.000000001s`, which keeps nanosecond precision. Reading also
//! accepts the ISO 8601 form (`PT1.5S`, `-PT5S`, `P0D`) that `chrono::Duration`
//! values were written as before.

use crate::prelude::*;

/// Formats a duration from its sign, whole seconds and nanoseconds.
pub(crate) fn format(negative: bool, secs: u64, nanos: u32) -> String {
    let sign = if negative && (secs != 0 || nanos != 0) {
        "-"
    } else {
        ""
    };
    if nanos == 0 {
        return format!("{sign}{secs}s");
    }
    let fraction = format!("{nanos:09}");
    format!("{sign}{secs}.{}s", fraction.trim_end_matches('0'))
}

/// Parses a stored duration into its sign, whole seconds and nanoseconds.
///
/// Returns `None` when the text is not a duration this module can read.
pub(crate) fn parse(text: &str) -> Option<(bool, u64, u32)> {
    let text = text.trim();
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let number = if let Some(iso) = rest.strip_prefix('P') {
        if iso == "0D" {
            return Some((false, 0, 0));
        }
        iso.strip_prefix('T')?.strip_suffix('S')?
    } else {
        rest.strip_suffix('s').unwrap_or(rest)
    };
    let (whole, fraction) = match number.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (number, ""),
    };
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if fraction.len() > 9 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let secs = whole.parse().ok()?;
    let nanos = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u32>().ok()? * 10u32.pow(9 - fraction.len() as u32)
    };
    Some((negative, secs, nanos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_whole_and_fractional_seconds() {
        assert_eq!(format(false, 0, 0), "0s");
        assert_eq!(format(false, 90, 0), "90s");
        assert_eq!(format(false, 1, 500_000_000), "1.5s");
        assert_eq!(format(true, 0, 1), "-0.000000001s");
        assert_eq!(format(true, 0, 0), "0s");
    }

    #[test]
    fn parses_canonical_legacy_and_iso_forms() {
        assert_eq!(parse("90s"), Some((false, 90, 0)));
        assert_eq!(parse("-1.5s"), Some((true, 1, 500_000_000)));
        assert_eq!(parse("42"), Some((false, 42, 0)));
        assert_eq!(parse("PT5S"), Some((false, 5, 0)));
        assert_eq!(parse("-PT1.25S"), Some((true, 1, 250_000_000)));
        assert_eq!(parse("P0D"), Some((false, 0, 0)));
    }

    #[test]
    fn rejects_malformed_text() {
        for text in [
            "",
            "s",
            "-",
            "1.s5",
            "1.0000000001s",
            "P1D",
            "PT5",
            "abc",
            "1e3s",
            "+5s",
        ] {
            assert_eq!(parse(text), None, "{text:?}");
        }
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn chrono_durations_round_trip_through_text() {
        use crate::traits::FromSQLiteValue;
        use crate::values::SQLiteValue;

        for duration in [
            chrono::Duration::zero(),
            chrono::Duration::seconds(5),
            chrono::Duration::milliseconds(1_500),
            chrono::Duration::milliseconds(-1_500),
            chrono::Duration::nanoseconds(-1),
            chrono::Duration::days(3) + chrono::Duration::nanoseconds(7),
        ] {
            let SQLiteValue::Text(text) = SQLiteValue::from(duration) else {
                panic!("durations are stored as text");
            };
            assert_eq!(
                chrono::Duration::from_sqlite_text(&text).unwrap(),
                duration,
                "{text}"
            );
        }
        assert_eq!(
            SQLiteValue::from(chrono::Duration::milliseconds(-1_500)),
            SQLiteValue::Text(Cow::Borrowed("-1.5s"))
        );
        // Text written by earlier versions, which used chrono's ISO 8601 form.
        assert_eq!(
            chrono::Duration::from_sqlite_text("-PT1.5S").unwrap(),
            chrono::Duration::milliseconds(-1_500)
        );
    }

    #[cfg(feature = "time")]
    #[test]
    fn time_durations_round_trip_through_text() {
        use crate::traits::FromSQLiteValue;
        use crate::values::SQLiteValue;

        for duration in [
            time::Duration::ZERO,
            time::Duration::seconds(5),
            time::Duration::milliseconds(1_500),
            time::Duration::milliseconds(-1_500),
            time::Duration::nanoseconds(-1),
            time::Duration::days(3) + time::Duration::nanoseconds(7),
        ] {
            let SQLiteValue::Text(text) = SQLiteValue::from(duration) else {
                panic!("durations are stored as text");
            };
            assert_eq!(
                time::Duration::from_sqlite_text(&text).unwrap(),
                duration,
                "{text}"
            );
        }
        // Text written by earlier versions, which kept whole seconds only.
        assert_eq!(
            time::Duration::from_sqlite_text("90s").unwrap(),
            time::Duration::seconds(90)
        );
    }
}
