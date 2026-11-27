//! Date and time values in the text `PostgreSQL` writes and reads.
//!
//! Drivers that bind and decode binary values (`postgres`, `tokio-postgres`)
//! do not use this. Text reaches drizzle from a relational query's JSON
//! (`json_build_object` writes ISO 8601, with `T` between date and time) and
//! from the AWS Data API (ISO `DateStyle`, with a space). Both write:
//!
//! - a year past 9999 with more digits: `10000-01-01`;
//! - a year before 1 AD with a ` BC` suffix: `0001-01-01 BC` is year 0 of
//!   the proleptic Gregorian calendar `chrono`, `time` and `jiff` use, as
//!   `PostgreSQL` numbers no year 0;
//! - an offset of whole hours as `+05`, and one with seconds as `+05:53:28`
//!   (a zone's local mean time before standard time, such as
//!   `Asia/Calcutta` before 1900).
//!
//! Values are written back in the same forms, so a parameter outside years
//! 1 to 9999 reaches the server as the value it is.

use crate::prelude::*;
#[cfg(feature = "aws-data-api")]
use core::fmt::Write as _;
use drizzle_core::error::DrizzleError;

/// A calendar date. `year` is astronomical: 0 is 1 BC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ymd {
    pub(crate) year: i32,
    pub(crate) month: u8,
    pub(crate) day: u8,
}

/// A time of day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Hms {
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) second: u8,
    pub(crate) nanosecond: u32,
}

/// A date and time, with the UTC offset in seconds when the text has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Timestamp {
    pub(crate) date: Ymd,
    pub(crate) time: Hms,
    pub(crate) offset: Option<i32>,
}

fn invalid(text: &str, kind: &str) -> DrizzleError {
    DrizzleError::ConversionError(format!("invalid PostgreSQL {kind} text {text:?}").into())
}

/// Reads ASCII digits off the front of `rest`.
struct Cursor<'a> {
    rest: &'a [u8],
}

impl Cursor<'_> {
    /// Reads `min..=max` digits; `None` when fewer than `min` are there.
    fn number(&mut self, min: usize, max: usize) -> Option<u32> {
        let len = self
            .rest
            .iter()
            .take(max)
            .take_while(|b| b.is_ascii_digit())
            .count();
        if len < min {
            return None;
        }
        let (digits, rest) = self.rest.split_at(len);
        self.rest = rest;
        Some(
            digits
                .iter()
                .fold(0, |n, digit| n * 10 + u32::from(digit - b'0')),
        )
    }

    fn eat(&mut self, byte: u8) -> bool {
        let matched = self.rest.first() == Some(&byte);
        if matched {
            self.rest = &self.rest[1..];
        }
        matched
    }

    fn two_digits(&mut self) -> Option<u8> {
        self.number(2, 2).and_then(|n| u8::try_from(n).ok())
    }

    /// `YYYY-MM-DD`, the year with four or more digits.
    fn date(&mut self) -> Option<(u32, u8, u8)> {
        let year = self.number(4, 7)?;
        if !self.eat(b'-') {
            return None;
        }
        let month = self.two_digits()?;
        if !self.eat(b'-') {
            return None;
        }
        Some((year, month, self.two_digits()?))
    }

    /// `HH:MM:SS[.fraction]`. Digits past nanoseconds are dropped.
    fn time(&mut self) -> Option<Hms> {
        let hour = self.two_digits()?;
        if !self.eat(b':') {
            return None;
        }
        let minute = self.two_digits()?;
        if !self.eat(b':') {
            return None;
        }
        let second = self.two_digits()?;
        let mut nanosecond = 0;
        if self.eat(b'.') {
            let len = self.rest.iter().take_while(|b| b.is_ascii_digit()).count();
            if len == 0 {
                return None;
            }
            let (digits, rest) = self.rest.split_at(len);
            self.rest = rest;
            for (place, digit) in digits.iter().take(9).enumerate() {
                nanosecond += u32::from(digit - b'0') * 10_u32.pow(8 - u32::try_from(place).ok()?);
            }
        }
        Some(Hms {
            hour,
            minute,
            second,
            nanosecond,
        })
    }

    /// `±HH[:MM[:SS]]` in seconds, or `None` when no offset follows.
    fn offset(&mut self) -> Result<Option<i32>, ()> {
        let sign = if self.eat(b'+') {
            1
        } else if self.eat(b'-') {
            -1
        } else {
            return Ok(None);
        };
        let mut seconds = i32::from(self.two_digits().ok_or(())?) * 3600;
        for unit in [60, 1] {
            if !self.eat(b':') {
                break;
            }
            seconds += i32::from(self.two_digits().ok_or(())?) * unit;
        }
        Ok(Some(sign * seconds))
    }
}

/// Splits a trailing ` BC` off `text`.
fn strip_era(text: &str) -> (&str, bool) {
    text.strip_suffix(" BC")
        .map_or((text, false), |text| (text, true))
}

/// The astronomical year of a `PostgreSQL` year: `1 BC` is 0.
fn astronomical_year(year: u32, bc: bool) -> Option<i32> {
    let year = i32::try_from(year).ok()?;
    if bc {
        (year >= 1).then(|| 1 - year)
    } else {
        Some(year)
    }
}

/// Parses a `date`: `YYYY-MM-DD[ BC]`.
pub(crate) fn parse_date(text: &str) -> Result<Ymd, DrizzleError> {
    let (body, bc) = strip_era(text);
    let mut cursor = Cursor {
        rest: body.as_bytes(),
    };
    cursor
        .date()
        .filter(|_| cursor.rest.is_empty())
        .and_then(|(year, month, day)| {
            Some(Ymd {
                year: astronomical_year(year, bc)?,
                month,
                day,
            })
        })
        .ok_or_else(|| invalid(text, "date"))
}

/// Parses a `time` or `timetz`: `HH:MM:SS[.fraction][±HH[:MM[:SS]]]`.
pub(crate) fn parse_time(text: &str) -> Result<(Hms, Option<i32>), DrizzleError> {
    let mut cursor = Cursor {
        rest: text.as_bytes(),
    };
    let time = cursor.time().ok_or_else(|| invalid(text, "time"))?;
    let offset = cursor.offset().map_err(|()| invalid(text, "time"))?;
    if !cursor.rest.is_empty() {
        return Err(invalid(text, "time"));
    }
    Ok((time, offset))
}

/// Parses a `timestamp` or `timestamptz`:
/// `YYYY-MM-DD[ T]HH:MM:SS[.fraction][±HH[:MM[:SS]]][ BC]`.
pub(crate) fn parse_timestamp(text: &str) -> Result<Timestamp, DrizzleError> {
    let (body, bc) = strip_era(text);
    let mut cursor = Cursor {
        rest: body.as_bytes(),
    };
    let parsed = (|| {
        let (year, month, day) = cursor.date()?;
        if !cursor.eat(b' ') && !cursor.eat(b'T') {
            return None;
        }
        let time = cursor.time()?;
        let offset = cursor.offset().ok()?;
        cursor.rest.is_empty().then_some(())?;
        Some(Timestamp {
            date: Ymd {
                year: astronomical_year(year, bc)?,
                month,
                day,
            },
            time,
            offset,
        })
    })();
    parsed.ok_or_else(|| invalid(text, "timestamp"))
}

/// The ` BC` suffix a date in `year` (astronomical) is written with.
#[cfg(feature = "aws-data-api")]
const fn era(year: i32) -> &'static str {
    if year < 1 { " BC" } else { "" }
}

#[cfg(feature = "aws-data-api")]
fn push_date(out: &mut String, date: Ymd) {
    let year = if date.year < 1 {
        1 - i64::from(date.year)
    } else {
        i64::from(date.year)
    };
    let _ = write!(out, "{year:04}-{:02}-{:02}", date.month, date.day);
}

/// Writes `time` as `HH:MM:SS[.fraction]`, trailing zeros of the fraction
/// trimmed.
#[cfg(feature = "aws-data-api")]
pub(crate) fn format_time(time: Hms) -> String {
    let mut out = String::with_capacity(18);
    // chrono keeps a leap second as a nanosecond count past one second;
    // PostgreSQL reads second 60 as the next second.
    let (second, nanosecond) = if time.nanosecond >= 1_000_000_000 {
        (time.second + 1, time.nanosecond - 1_000_000_000)
    } else {
        (time.second, time.nanosecond)
    };
    let _ = write!(out, "{:02}:{:02}:{second:02}", time.hour, time.minute);
    if nanosecond != 0 {
        let fraction = format!("{nanosecond:09}");
        out.push('.');
        out.push_str(fraction.trim_end_matches('0'));
    }
    out
}

/// Writes `date` as `YYYY-MM-DD[ BC]`.
#[cfg(feature = "aws-data-api")]
pub(crate) fn format_date(date: Ymd) -> String {
    let mut out = String::with_capacity(13);
    push_date(&mut out, date);
    out.push_str(era(date.year));
    out
}

/// Writes a date and time as `YYYY-MM-DD HH:MM:SS[.fraction][ BC]`.
#[cfg(feature = "aws-data-api")]
pub(crate) fn format_timestamp(date: Ymd, time: Hms) -> String {
    let mut out = String::with_capacity(32);
    push_date(&mut out, date);
    out.push(' ');
    out.push_str(&format_time(time));
    out.push_str(era(date.year));
    out
}

/// The UTC date and time of a local date and time `offset` seconds east of
/// UTC. Calendar arithmetic on plain integers, so the result may fall
/// outside the range of the type the value came from (`time` ends at year
/// 9999).
#[cfg(all(feature = "time", feature = "aws-data-api"))]
pub(crate) fn to_utc(date: Ymd, time: Hms, offset: i32) -> (Ymd, Hms) {
    // Days since 1970-01-01 in the proleptic Gregorian calendar, after
    // Howard Hinnant's `days_from_civil`.
    let (month, day) = (i64::from(date.month), i64::from(date.day));
    let year = i64::from(date.year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;

    let seconds = days * 86_400
        + i64::from(time.hour) * 3600
        + i64::from(time.minute) * 60
        + i64::from(time.second)
        - i64::from(offset);
    let (days, second_of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));

    // Hinnant's `civil_from_days`.
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    let narrow = |value: i64| u8::try_from(value).expect("a calendar field fits u8");
    (
        Ymd {
            year: i32::try_from(year).expect("a day away from an i32 year fits i32"),
            month: narrow(month),
            day: narrow(day),
        },
        Hms {
            hour: narrow(second_of_day / 3600),
            minute: narrow(second_of_day / 60 % 60),
            second: narrow(second_of_day % 60),
            nanosecond: time.nanosecond,
        },
    )
}

/// A date or time type read from `PostgreSQL`'s text.
///
/// Used by generated relational query decoders and the AWS Data API row
/// decoders; not meant to be implemented outside drizzle.
#[doc(hidden)]
pub trait FromTemporalText: Sized {
    /// Reads `Self` from `text`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when `text` is not a value
    /// of the type's `PostgreSQL` text form, or is outside the range `Self`
    /// can hold.
    fn from_temporal_text(text: &str) -> Result<Self, DrizzleError>;
}

/// Decodes a date or time field of a relational query's JSON.
///
/// # Errors
///
/// Returns [`DrizzleError::ConversionError`] when the value is not a JSON
/// string or the string is not a value of `T`.
#[cfg(feature = "query")]
#[doc(hidden)]
pub fn decode_projected_temporal<T: FromTemporalText>(
    projected: &serde_json::Value,
) -> Result<T, DrizzleError> {
    let text = projected.as_str().ok_or_else(|| {
        DrizzleError::ConversionError("a date or time value is not a JSON string".into())
    })?;
    T::from_temporal_text(text)
}

fn out_of_range(text: &str, target: &str) -> DrizzleError {
    DrizzleError::ConversionError(format!("{text:?} is out of range for {target}").into())
}

/// A time of day read with an offset (a `timetz`) would lose the offset.
fn reject_offset(text: &str, offset: Option<i32>, target: &str) -> Result<(), DrizzleError> {
    match offset {
        None => Ok(()),
        Some(_) => Err(DrizzleError::ConversionError(
            format!("{text:?} has a UTC offset, which {target} cannot hold").into(),
        )),
    }
}

#[cfg(feature = "chrono")]
mod chrono_impls {
    use super::{
        DrizzleError, FromTemporalText, Hms, Timestamp, Ymd, out_of_range, parse_date, parse_time,
        parse_timestamp, reject_offset,
    };
    use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
    #[cfg(feature = "aws-data-api")]
    use chrono::{Datelike, Timelike};

    #[cfg(feature = "aws-data-api")]
    pub(crate) fn ymd(date: NaiveDate) -> Ymd {
        Ymd {
            year: date.year(),
            month: u8::try_from(date.month()).expect("a month fits u8"),
            day: u8::try_from(date.day()).expect("a day fits u8"),
        }
    }

    #[cfg(feature = "aws-data-api")]
    pub(crate) fn hms(time: NaiveTime) -> Hms {
        Hms {
            hour: u8::try_from(time.hour()).expect("an hour fits u8"),
            minute: u8::try_from(time.minute()).expect("a minute fits u8"),
            second: u8::try_from(time.second()).expect("a second fits u8"),
            nanosecond: time.nanosecond(),
        }
    }

    fn date(text: &str, date: Ymd) -> Result<NaiveDate, DrizzleError> {
        NaiveDate::from_ymd_opt(date.year, date.month.into(), date.day.into())
            .ok_or_else(|| out_of_range(text, "chrono::NaiveDate"))
    }

    fn time(text: &str, time: Hms) -> Result<NaiveTime, DrizzleError> {
        NaiveTime::from_hms_nano_opt(
            time.hour.into(),
            time.minute.into(),
            time.second.into(),
            time.nanosecond,
        )
        .ok_or_else(|| out_of_range(text, "chrono::NaiveTime"))
    }

    fn zoned(text: &str, value: Timestamp) -> Result<DateTime<FixedOffset>, DrizzleError> {
        let local = NaiveDateTime::new(date(text, value.date)?, time(text, value.time)?);
        FixedOffset::east_opt(value.offset.unwrap_or(0))
            .and_then(|offset| local.and_local_timezone(offset).single())
            .ok_or_else(|| out_of_range(text, "chrono::DateTime"))
    }

    impl FromTemporalText for NaiveDate {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            date(text, parse_date(text)?)
        }
    }

    impl FromTemporalText for NaiveTime {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            let (value, offset) = parse_time(text)?;
            reject_offset(text, offset, "chrono::NaiveTime")?;
            time(text, value)
        }
    }

    impl FromTemporalText for NaiveDateTime {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            let value = parse_timestamp(text)?;
            reject_offset(text, value.offset, "chrono::NaiveDateTime")?;
            Ok(Self::new(date(text, value.date)?, time(text, value.time)?))
        }
    }

    /// Text without an offset is UTC.
    impl FromTemporalText for DateTime<FixedOffset> {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            zoned(text, parse_timestamp(text)?)
        }
    }

    /// Text without an offset is UTC.
    impl FromTemporalText for DateTime<Utc> {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            zoned(text, parse_timestamp(text)?).map(|value| value.with_timezone(&Utc))
        }
    }
}

#[cfg(all(feature = "chrono", feature = "aws-data-api"))]
pub(crate) use chrono_impls::{hms as chrono_hms, ymd as chrono_ymd};

#[cfg(feature = "time")]
mod time_impls {
    use super::{
        DrizzleError, FromTemporalText, Hms, Timestamp, Ymd, out_of_range, parse_date, parse_time,
        parse_timestamp, reject_offset,
    };
    use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

    #[cfg(feature = "aws-data-api")]
    pub(crate) fn ymd(date: Date) -> Ymd {
        Ymd {
            year: date.year(),
            month: u8::from(date.month()),
            day: date.day(),
        }
    }

    #[cfg(feature = "aws-data-api")]
    pub(crate) fn hms(time: Time) -> Hms {
        Hms {
            hour: time.hour(),
            minute: time.minute(),
            second: time.second(),
            nanosecond: time.nanosecond(),
        }
    }

    fn date(text: &str, date: Ymd) -> Result<Date, DrizzleError> {
        Month::try_from(date.month)
            .ok()
            .and_then(|month| Date::from_calendar_date(date.year, month, date.day).ok())
            .ok_or_else(|| out_of_range(text, "time::Date"))
    }

    fn time(text: &str, time: Hms) -> Result<Time, DrizzleError> {
        Time::from_hms_nano(time.hour, time.minute, time.second, time.nanosecond)
            .map_err(|_| out_of_range(text, "time::Time"))
    }

    fn primitive(text: &str, value: Timestamp) -> Result<PrimitiveDateTime, DrizzleError> {
        Ok(PrimitiveDateTime::new(
            date(text, value.date)?,
            time(text, value.time)?,
        ))
    }

    impl FromTemporalText for Date {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            date(text, parse_date(text)?)
        }
    }

    impl FromTemporalText for Time {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            let (value, offset) = parse_time(text)?;
            reject_offset(text, offset, "time::Time")?;
            time(text, value)
        }
    }

    impl FromTemporalText for PrimitiveDateTime {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            let value = parse_timestamp(text)?;
            reject_offset(text, value.offset, "time::PrimitiveDateTime")?;
            primitive(text, value)
        }
    }

    /// Text without an offset is UTC.
    impl FromTemporalText for OffsetDateTime {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            let value = parse_timestamp(text)?;
            let offset = UtcOffset::from_whole_seconds(value.offset.unwrap_or(0))
                .map_err(|_| out_of_range(text, "time::OffsetDateTime"))?;
            Ok(primitive(text, value)?.assume_offset(offset))
        }
    }
}

#[cfg(all(feature = "time", feature = "aws-data-api"))]
pub(crate) use time_impls::{hms as time_hms, ymd as time_ymd};

#[cfg(feature = "jiff")]
mod jiff_impls {
    use super::{
        DrizzleError, FromTemporalText, Hms, Ymd, out_of_range, parse_date, parse_time,
        parse_timestamp, reject_offset,
    };
    use jiff::civil::{Date, DateTime, Time};
    use jiff::tz::Offset;

    #[cfg(feature = "aws-data-api")]
    pub(crate) fn ymd(date: Date) -> Ymd {
        Ymd {
            year: i32::from(date.year()),
            month: u8::try_from(date.month()).expect("a month is positive"),
            day: u8::try_from(date.day()).expect("a day is positive"),
        }
    }

    #[cfg(feature = "aws-data-api")]
    pub(crate) fn hms(time: Time) -> Hms {
        Hms {
            hour: u8::try_from(time.hour()).expect("an hour is not negative"),
            minute: u8::try_from(time.minute()).expect("a minute is not negative"),
            second: u8::try_from(time.second()).expect("a second is not negative"),
            nanosecond: u32::try_from(time.subsec_nanosecond())
                .expect("a nanosecond is not negative"),
        }
    }

    fn date(text: &str, date: Ymd) -> Result<Date, DrizzleError> {
        let year = i16::try_from(date.year).map_err(|_| out_of_range(text, "jiff::civil::Date"))?;
        Date::new(year, date.month.cast_signed(), date.day.cast_signed())
            .map_err(|_| out_of_range(text, "jiff::civil::Date"))
    }

    fn time(text: &str, time: Hms) -> Result<Time, DrizzleError> {
        Time::new(
            time.hour.cast_signed(),
            time.minute.cast_signed(),
            time.second.cast_signed(),
            i32::try_from(time.nanosecond).map_err(|_| out_of_range(text, "jiff::civil::Time"))?,
        )
        .map_err(|_| out_of_range(text, "jiff::civil::Time"))
    }

    impl FromTemporalText for Date {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            date(text, parse_date(text)?)
        }
    }

    impl FromTemporalText for Time {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            let (value, offset) = parse_time(text)?;
            reject_offset(text, offset, "jiff::civil::Time")?;
            time(text, value)
        }
    }

    impl FromTemporalText for DateTime {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            let value = parse_timestamp(text)?;
            reject_offset(text, value.offset, "jiff::civil::DateTime")?;
            Ok(Self::from_parts(
                date(text, value.date)?,
                time(text, value.time)?,
            ))
        }
    }

    /// Text without an offset is UTC.
    impl FromTemporalText for jiff::Timestamp {
        fn from_temporal_text(text: &str) -> Result<Self, DrizzleError> {
            let value = parse_timestamp(text)?;
            let civil = DateTime::from_parts(date(text, value.date)?, time(text, value.time)?);
            Offset::from_seconds(value.offset.unwrap_or(0))
                .and_then(|offset| offset.to_timestamp(civil))
                .map_err(|_| out_of_range(text, "jiff::Timestamp"))
        }
    }
}

#[cfg(all(feature = "jiff", feature = "aws-data-api"))]
pub(crate) use jiff_impls::{hms as jiff_hms, ymd as jiff_ymd};

#[cfg(test)]
mod tests {
    use super::*;

    const fn ymd(year: i32, month: u8, day: u8) -> Ymd {
        Ymd { year, month, day }
    }

    const fn hms(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Hms {
        Hms {
            hour,
            minute,
            second,
            nanosecond,
        }
    }

    #[test]
    fn dates_outside_years_1_to_9999_round_trip() {
        for (text, date) in [
            ("0001-01-01", ymd(1, 1, 1)),
            ("0099-12-31", ymd(99, 12, 31)),
            ("9999-12-31", ymd(9999, 12, 31)),
            ("10000-01-01", ymd(10000, 1, 1)),
            ("0001-01-01 BC", ymd(0, 1, 1)),
            ("0044-03-15 BC", ymd(-43, 3, 15)),
        ] {
            assert_eq!(parse_date(text).unwrap(), date, "{text}");
            #[cfg(feature = "aws-data-api")]
            assert_eq!(format_date(date), text);
        }
        for text in [
            "+10000-01-01",
            "-0001-01-01",
            "0000-01-01 BC",
            "01-01-01",
            "2020-1-01",
        ] {
            assert!(parse_date(text).is_err(), "{text}");
        }
    }

    #[test]
    fn timestamps_read_both_separators_offsets_and_eras() {
        let parsed = parse_timestamp("0001-01-01T00:17:30+00:17:30 BC").unwrap();
        assert_eq!(
            parsed,
            Timestamp {
                date: ymd(0, 1, 1),
                time: hms(0, 17, 30, 0),
                offset: Some(17 * 60 + 30),
            }
        );
        let parsed = parse_timestamp("1800-01-01 05:53:28.25+05:53:28").unwrap();
        assert_eq!(parsed.time, hms(5, 53, 28, 250_000_000));
        assert_eq!(parsed.offset, Some(5 * 3600 + 53 * 60 + 28));
        assert_eq!(
            parse_timestamp("2020-01-01 00:00:00-08").unwrap().offset,
            Some(-8 * 3600)
        );
        assert_eq!(
            parse_timestamp("10000-01-01T01:02:03.000004").unwrap(),
            Timestamp {
                date: ymd(10000, 1, 1),
                time: hms(1, 2, 3, 4_000),
                offset: None,
            }
        );
        for text in [
            "2020-01-01",
            "2020-01-01 1:02:03",
            "2020-01-01 01:02:03Z",
            "2020-01-01 01:02:03.",
        ] {
            assert!(parse_timestamp(text).is_err(), "{text}");
        }
    }

    #[cfg(feature = "aws-data-api")]
    #[test]
    fn times_pad_fields_and_trim_fractions() {
        assert_eq!(format_time(hms(1, 2, 3, 4_000)), "01:02:03.000004");
        assert_eq!(format_time(hms(23, 59, 59, 500_000_000)), "23:59:59.5");
        assert_eq!(format_time(hms(0, 0, 0, 0)), "00:00:00");
        assert_eq!(
            format_timestamp(ymd(-43, 3, 15), hms(12, 0, 0, 250_000_000)),
            "0044-03-15 12:00:00.25 BC"
        );
        assert_eq!(
            parse_time("12:34:56.5+05:30").unwrap(),
            (hms(12, 34, 56, 500_000_000), Some(5 * 3600 + 30 * 60))
        );
        assert_eq!(parse_time("01:02:03").unwrap(), (hms(1, 2, 3, 0), None));
    }

    #[cfg(all(feature = "time", feature = "aws-data-api"))]
    #[test]
    fn utc_shifts_across_days_months_and_eras() {
        assert_eq!(
            to_utc(ymd(2020, 3, 1), hms(0, 30, 0, 7), 3600),
            (ymd(2020, 2, 29), hms(23, 30, 0, 7))
        );
        assert_eq!(
            to_utc(ymd(1, 1, 1), hms(0, 0, 0, 0), 3600),
            (ymd(0, 12, 31), hms(23, 0, 0, 0))
        );
        assert_eq!(
            to_utc(ymd(9999, 12, 31), hms(23, 0, 0, 0), -5 * 3600),
            (ymd(10000, 1, 1), hms(4, 0, 0, 0))
        );
        assert_eq!(
            to_utc(ymd(1800, 1, 1), hms(5, 53, 28, 0), 5 * 3600 + 53 * 60 + 28),
            (ymd(1800, 1, 1), hms(0, 0, 0, 0))
        );
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn chrono_reads_postgres_text() {
        use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};

        assert_eq!(
            NaiveDate::from_temporal_text("10000-01-01").unwrap(),
            NaiveDate::from_ymd_opt(10000, 1, 1).unwrap()
        );
        assert_eq!(
            NaiveDate::from_temporal_text("0001-01-01 BC").unwrap(),
            NaiveDate::from_ymd_opt(0, 1, 1).unwrap()
        );
        assert_eq!(
            NaiveDate::from_temporal_text("0099-12-31").unwrap(),
            NaiveDate::from_ymd_opt(99, 12, 31).unwrap()
        );
        assert_eq!(
            NaiveDateTime::from_temporal_text("0001-01-01T00:00:00.25 BC").unwrap(),
            NaiveDate::from_ymd_opt(0, 1, 1)
                .unwrap()
                .and_hms_milli_opt(0, 0, 0, 250)
                .unwrap()
        );
        let lmt =
            DateTime::<FixedOffset>::from_temporal_text("1800-01-01T05:53:28+05:53:28").unwrap();
        assert_eq!(lmt.offset().local_minus_utc(), 5 * 3600 + 53 * 60 + 28);
        assert_eq!(
            DateTime::<Utc>::from_temporal_text("1800-01-01T05:53:28+05:53:28").unwrap(),
            "1800-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(
            DateTime::<Utc>::from_temporal_text("2020-01-01 00:00:00").unwrap(),
            "2020-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert!(NaiveTime::from_temporal_text("12:34:56+05").is_err());
        assert!(NaiveDateTime::from_temporal_text("2020-01-01T00:00:00+05").is_err());
        assert!(NaiveDate::from_temporal_text("2020-02-30").is_err());
    }

    #[cfg(feature = "time")]
    #[test]
    fn time_reads_postgres_text() {
        use time::macros::{date, datetime, time};
        use time::{Date, OffsetDateTime, PrimitiveDateTime, Time};

        assert_eq!(
            Date::from_temporal_text("0001-01-01 BC").unwrap(),
            date!(0000 - 01 - 01)
        );
        assert_eq!(
            PrimitiveDateTime::from_temporal_text("0099-06-01T12:00:00").unwrap(),
            datetime!(0099-06-01 12:00)
        );
        assert_eq!(
            OffsetDateTime::from_temporal_text("1800-01-01T05:53:28+05:53:28").unwrap(),
            datetime!(1800-01-01 0:00 UTC)
        );
        assert_eq!(
            Time::from_temporal_text("01:02:03.000004").unwrap(),
            time!(1:02:03.000004)
        );
        assert!(Time::from_temporal_text("12:34:56+05").is_err());
    }

    #[cfg(feature = "jiff")]
    #[test]
    fn jiff_reads_postgres_text() {
        use jiff::civil::{Date, DateTime, Time, date};

        assert_eq!(
            Date::from_temporal_text("0001-01-01 BC").unwrap(),
            date(0, 1, 1)
        );
        assert_eq!(
            DateTime::from_temporal_text("0099-12-31T23:59:59.5").unwrap(),
            date(99, 12, 31).at(23, 59, 59, 500_000_000)
        );
        assert_eq!(
            jiff::Timestamp::from_temporal_text("1800-01-01T05:53:28+05:53:28").unwrap(),
            "1800-01-01T00:00:00Z".parse::<jiff::Timestamp>().unwrap()
        );
        // jiff holds years -9999 to 9999.
        assert!(Date::from_temporal_text("10000-01-01").is_err());
        // A `timetz` read as a time of day would drop its offset.
        assert!(Time::from_temporal_text("12:34:56+05").is_err());
    }
}
