//! Writing bound values as SQL literals, for statements that run without
//! parameters (`inline_sql`, `try_generate_script`, the `drizzle seed` CLI).
//!
//! Every literal reads the same whatever the server's settings: PostgreSQL
//! strings only use `E'...'` when they hold a backslash, so
//! `standard_conforming_strings` does not matter; bytea is written with
//! `decode(..., 'hex')`; MySQL text with a backslash is written in hex, so
//! `NO_BACKSLASH_ESCAPES` does not matter either, and carries a `_utf8mb4`
//! introducer, so the connection character set does not.

use core::fmt::Write as _;

/// Why a value has no literal form.
pub(crate) type LiteralError = String;

/// `'text'`, with embedded quotes doubled.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for character in text.chars() {
        if character == '\'' {
            out.push('\'');
        }
        out.push(character);
    }
    out.push('\'');
    out
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The shortest decimal that reads back as the same `f64`, or `None` for
/// NaN and infinities.
fn finite(value: f64) -> Option<String> {
    // `Debug` prints the shortest round-trip form and always keeps a `.` or
    // an exponent, so `1.0` stays a real number.
    value.is_finite().then(|| format!("{value:?}"))
}

#[cfg(feature = "sqlite")]
pub(crate) fn sqlite(
    value: &drizzle_sqlite::values::OwnedSQLiteValue,
) -> Result<String, LiteralError> {
    use drizzle_sqlite::values::OwnedSQLiteValue as V;
    Ok(match value {
        V::Null => "NULL".to_owned(),
        V::Integer(value) => value.to_string(),
        // SQLite stores a bound NaN as NULL, and reads 9e999 as infinity.
        V::Real(value) if value.is_nan() => "NULL".to_owned(),
        V::Real(value) if value.is_infinite() => if value.is_sign_positive() {
            "9e999"
        } else {
            "-9e999"
        }
        .to_owned(),
        V::Real(value) => finite(*value).unwrap_or_default(),
        // A string literal ends at a NUL character; a blob cast keeps it.
        V::Text(text) if text.contains('\0') => {
            format!("CAST(X'{}' AS TEXT)", hex(text.as_bytes()))
        }
        V::Text(text) => quoted(text),
        V::Blob(bytes) => format!("X'{}'", hex(bytes)),
    })
}

#[cfg(feature = "postgres")]
pub(crate) fn postgres(
    value: &drizzle_postgres::values::OwnedPostgresValue,
) -> Result<String, LiteralError> {
    use drizzle_postgres::values::OwnedPostgresValue as V;
    Ok(match value {
        V::Null => "NULL".to_owned(),
        V::Smallint(value) => value.to_string(),
        V::Integer(value) => value.to_string(),
        V::Bigint(value) => value.to_string(),
        V::Real(value) => float_postgres(f64::from(*value), "real"),
        V::DoublePrecision(value) => float_postgres(*value, "double precision"),
        V::Boolean(value) => if *value { "TRUE" } else { "FALSE" }.to_owned(),
        V::Text(text) if text.contains('\0') => {
            return Err("PostgreSQL text cannot contain a NUL character".to_owned());
        }
        V::Text(text) if text.contains('\\') => {
            // An escape string reads the same whatever
            // `standard_conforming_strings` is set to.
            format!("E{}", quoted(&text.replace('\\', "\\\\")))
        }
        V::Text(text) => quoted(text),
        V::Bytea(bytes) => format!("decode('{}', 'hex')", hex(bytes)),
        #[cfg(feature = "chrono")]
        V::Date(date) => format!("'{}'::date", date.format("%Y-%m-%d")),
        #[cfg(feature = "chrono")]
        V::Time(time) => format!("'{}'::time", time.format("%H:%M:%S%.f")),
        #[cfg(feature = "chrono")]
        V::Timestamp(timestamp) => {
            format!("'{}'::timestamp", timestamp.format("%Y-%m-%d %H:%M:%S%.f"))
        }
        #[cfg(feature = "chrono")]
        V::TimestampTz(timestamp) => format!(
            "'{}'::timestamptz",
            timestamp.format("%Y-%m-%d %H:%M:%S%.f%:z")
        ),
        #[allow(unreachable_patterns)]
        other => return Err(format!("no SQL literal for the PostgreSQL value {other:?}")),
    })
}

#[cfg(feature = "postgres")]
fn float_postgres(value: f64, sql_type: &str) -> String {
    finite(value).unwrap_or_else(|| {
        let special = if value.is_nan() {
            "NaN"
        } else if value.is_sign_positive() {
            "Infinity"
        } else {
            "-Infinity"
        };
        format!("'{special}'::{sql_type}")
    })
}

#[cfg(feature = "mysql")]
pub(crate) fn mysql(
    value: &drizzle_mysql::values::OwnedMySQLValue,
) -> Result<String, LiteralError> {
    use drizzle_mysql::values::OwnedMySQLValue as V;
    Ok(match value {
        V::Null => "NULL".to_owned(),
        V::Int(value) => value.to_string(),
        V::UInt(value) => value.to_string(),
        V::Float(value) => finite(f64::from(*value))
            .ok_or_else(|| format!("MySQL cannot store the float {value}"))?,
        V::Double(value) => {
            finite(*value).ok_or_else(|| format!("MySQL cannot store the double {value}"))?
        }
        V::Bytes(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) if !text.contains(['\\', '\0']) => format!("_utf8mb4{}", quoted(text)),
            Ok(_) => format!("_utf8mb4 X'{}'", hex(bytes)),
            Err(_) => format!("X'{}'", hex(bytes)),
        },
        V::Date {
            year,
            month,
            day,
            hour,
            minute,
            second,
            microseconds,
        } => {
            let mut out = format!("'{year:04}-{month:02}-{day:02}");
            if (*hour, *minute, *second, *microseconds) != (0, 0, 0, 0) {
                let _ = write!(out, " {hour:02}:{minute:02}:{second:02}");
                if *microseconds != 0 {
                    let _ = write!(out, ".{microseconds:06}");
                }
            }
            out.push('\'');
            out
        }
        V::Time {
            negative,
            days,
            hours,
            minutes,
            seconds,
            microseconds,
        } => {
            let total_hours = u64::from(*days) * 24 + u64::from(*hours);
            let sign = if *negative { "-" } else { "" };
            let mut out = format!("'{sign}{total_hours:02}:{minutes:02}:{seconds:02}");
            if *microseconds != 0 {
                let _ = write!(out, ".{microseconds:06}");
            }
            out.push('\'');
            out
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_keep_a_decimal_point_and_round_trip() {
        for value in [1.0, 0.1, -2.5, 1e-7, 1.5e300, 123_456_789.0] {
            let text = finite(value).unwrap();
            assert!(text.contains(['.', 'e']), "{text}");
            assert_eq!(text.parse::<f64>().unwrap(), value);
        }
        assert_eq!(finite(f64::NAN), None);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn sqlite_literals() {
        use drizzle_sqlite::values::OwnedSQLiteValue as V;
        assert_eq!(sqlite(&V::Text("it's".into())).unwrap(), "'it''s'");
        assert_eq!(sqlite(&V::Text("a\\b".into())).unwrap(), "'a\\b'");
        assert_eq!(
            sqlite(&V::Text("a\0b".into())).unwrap(),
            "CAST(X'610062' AS TEXT)"
        );
        assert_eq!(sqlite(&V::Blob(vec![0, 255].into())).unwrap(), "X'00ff'");
        assert_eq!(sqlite(&V::Real(f64::NAN)).unwrap(), "NULL");
        assert_eq!(sqlite(&V::Integer(-3)).unwrap(), "-3");
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn postgres_literals() {
        use drizzle_postgres::values::OwnedPostgresValue as V;
        assert_eq!(postgres(&V::Text("it's".into())).unwrap(), "'it''s'");
        assert_eq!(postgres(&V::Text("a\\b'".into())).unwrap(), "E'a\\\\b'''");
        assert!(postgres(&V::Text("a\0".into())).is_err());
        assert_eq!(
            postgres(&V::Bytea(vec![1, 171])).unwrap(),
            "decode('01ab', 'hex')"
        );
        assert_eq!(
            postgres(&V::DoublePrecision(f64::NEG_INFINITY)).unwrap(),
            "'-Infinity'::double precision"
        );
        assert_eq!(postgres(&V::Boolean(true)).unwrap(), "TRUE");
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_literals() {
        use drizzle_mysql::values::OwnedMySQLValue as V;
        assert_eq!(
            mysql(&V::Bytes(b"it's".to_vec())).unwrap(),
            "_utf8mb4'it''s'"
        );
        assert_eq!(
            mysql(&V::Bytes(b"a\\b".to_vec())).unwrap(),
            "_utf8mb4 X'615c62'"
        );
        assert_eq!(mysql(&V::Bytes(vec![0xff, 0])).unwrap(), "X'ff00'");
        assert!(mysql(&V::Double(f64::NAN)).is_err());
        let date = V::Date {
            year: 2024,
            month: 2,
            day: 29,
            hour: 0,
            minute: 0,
            second: 0,
            microseconds: 0,
        };
        assert_eq!(mysql(&date).unwrap(), "'2024-02-29'");
        let time = V::Time {
            negative: true,
            days: 1,
            hours: 2,
            minutes: 3,
            seconds: 4,
            microseconds: 50,
        };
        assert_eq!(mysql(&time).unwrap(), "'-26:03:04.000050'");
    }
}
