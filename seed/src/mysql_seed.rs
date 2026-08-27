use std::borrow::Cow;

use drizzle_core::{ColumnRef, SQL, Token};
use drizzle_mysql::values::OwnedMySQLValue;
use drizzle_types::mysql::MySQLTypeCategory;

use crate::{
    GeneratedChunk, MySQLSeedStatement, SeedError, SeedValue, build_insert_sql, identity::TableId,
    inference,
};

pub(crate) fn build_statement(chunk: &GeneratedChunk<'_>) -> Result<MySQLSeedStatement, SeedError> {
    let table = TableId::from_ref(chunk.table);
    let rows: Result<Vec<Vec<SQL<'static, OwnedMySQLValue>>>, SeedError> = chunk
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .zip(chunk.table.columns)
                .map(|(value, column)| value_to_sql(value, column, table))
                .collect()
        })
        .collect();

    Ok(MySQLSeedStatement {
        inner: build_insert_sql(chunk.table, &rows?),
    })
}

fn value_to_sql(
    value: &SeedValue,
    column: &ColumnRef,
    table: TableId,
) -> Result<SQL<'static, OwnedMySQLValue>, SeedError> {
    let category = MySQLTypeCategory::from_sql_type(column.sql_type);
    let invalid = |reason: String| SeedError::InvalidValue {
        table: table.to_string(),
        column: column.name.to_string(),
        reason,
    };

    let owned = match value {
        SeedValue::Default => return Ok(SQL::token(Token::DEFAULT)),
        SeedValue::CurrentTime => return Ok(SQL::raw("CURRENT_TIMESTAMP")),
        SeedValue::Null => OwnedMySQLValue::Null,
        SeedValue::Integer(value) => {
            integer_value(*value, category).map_err(|reason| invalid(reason.to_string()))?
        }
        SeedValue::Float(value) => float_value(*value, category)
            .ok_or_else(|| invalid(format!("{value} is outside the declared floating domain")))?,
        SeedValue::Text(value) => text_value(value, column, category)
            .ok_or_else(|| invalid(format!("{value:?} is outside the declared SQL domain")))?,
        SeedValue::Bool(value) => OwnedMySQLValue::Int(i64::from(*value)),
        SeedValue::Blob(value) => OwnedMySQLValue::Bytes(value.clone()),
    };
    Ok(SQL::param(Cow::Owned(owned)))
}

fn integer_value(value: i64, category: MySQLTypeCategory) -> Result<OwnedMySQLValue, &'static str> {
    let signed_bounds = match category {
        MySQLTypeCategory::TinyInt => Some((i64::from(i8::MIN), i64::from(i8::MAX))),
        MySQLTypeCategory::SmallInt => Some((i64::from(i16::MIN), i64::from(i16::MAX))),
        MySQLTypeCategory::MediumInt => Some((-8_388_608, 8_388_607)),
        MySQLTypeCategory::Int => Some((i64::from(i32::MIN), i64::from(i32::MAX))),
        MySQLTypeCategory::BigInt => Some((i64::MIN, i64::MAX)),
        _ => None,
    };
    if let Some((minimum, maximum)) = signed_bounds {
        return (minimum..=maximum)
            .contains(&value)
            .then_some(OwnedMySQLValue::Int(value))
            .ok_or("integer is outside the declared signed MySQL type range");
    }

    let unsigned_max = match category {
        MySQLTypeCategory::TinyIntUnsigned => Some(u64::from(u8::MAX)),
        MySQLTypeCategory::SmallIntUnsigned => Some(u64::from(u16::MAX)),
        MySQLTypeCategory::MediumIntUnsigned => Some(16_777_215),
        MySQLTypeCategory::IntUnsigned => Some(u64::from(u32::MAX)),
        MySQLTypeCategory::BigIntUnsigned => Some(u64::MAX),
        MySQLTypeCategory::Year => Some(2155),
        _ => None,
    };
    if let Some(maximum) = unsigned_max {
        let unsigned = u64::try_from(value)
            .map_err(|_| "negative integer cannot be bound to an unsigned MySQL type")?;
        if category == MySQLTypeCategory::Year
            && unsigned != 0
            && !(1901..=2155).contains(&unsigned)
        {
            return Err("YEAR must be in 1901..=2155");
        }
        return (unsigned <= maximum)
            .then_some(OwnedMySQLValue::UInt(unsigned))
            .ok_or("integer is outside the declared unsigned MySQL type range");
    }

    Ok(OwnedMySQLValue::Int(value))
}

fn float_value(value: f64, category: MySQLTypeCategory) -> Option<OwnedMySQLValue> {
    if !value.is_finite() {
        return None;
    }
    match category {
        MySQLTypeCategory::Float
            if value <= f64::from(f32::MAX) && value >= f64::from(f32::MIN) =>
        {
            Some(OwnedMySQLValue::Float(value as f32))
        }
        MySQLTypeCategory::Float => None,
        MySQLTypeCategory::Decimal => Some(OwnedMySQLValue::Bytes(value.to_string().into_bytes())),
        _ => Some(OwnedMySQLValue::Double(value)),
    }
}

fn text_value(
    value: &str,
    column: &ColumnRef,
    category: MySQLTypeCategory,
) -> Option<OwnedMySQLValue> {
    match category {
        MySQLTypeCategory::Date => parse_date(value).map(DateParts::into_value),
        MySQLTypeCategory::DateTime => parse_datetime(value).map(DateParts::into_value),
        MySQLTypeCategory::Timestamp => parse_timestamp(value).map(DateParts::into_value),
        MySQLTypeCategory::Time => parse_time(value),
        MySQLTypeCategory::Enum => {
            let allowed = inference::mysql_inline_labels(column.sql_type, "ENUM")?;
            allowed
                .iter()
                .any(|candidate| candidate == value)
                .then(|| OwnedMySQLValue::Bytes(value.as_bytes().to_vec()))
        }
        MySQLTypeCategory::Set => {
            let allowed = inference::mysql_inline_labels(column.sql_type, "SET")?;
            (value.is_empty()
                || value
                    .split(',')
                    .all(|member| allowed.iter().any(|candidate| candidate == member)))
            .then(|| OwnedMySQLValue::Bytes(value.as_bytes().to_vec()))
        }
        _ => Some(OwnedMySQLValue::Bytes(value.as_bytes().to_vec())),
    }
}

struct DateParts {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    microseconds: u32,
}

impl DateParts {
    fn into_value(self) -> OwnedMySQLValue {
        OwnedMySQLValue::Date {
            year: self.year,
            month: self.month,
            day: self.day,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
            microseconds: self.microseconds,
        }
    }
}

fn parse_date(value: &str) -> Option<DateParts> {
    let mut parts = value.split('-');
    let year: u16 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(1000..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
    {
        return None;
    }
    Some(DateParts {
        year,
        month,
        day,
        hour: 0,
        minute: 0,
        second: 0,
        microseconds: 0,
    })
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => 31,
    }
}

fn parse_datetime(value: &str) -> Option<DateParts> {
    let (date, time) = value.split_once(' ')?;
    let mut parts = parse_date(date)?;
    let (hour, minute, second, microseconds) = parse_clock(time)?;
    if hour > 23 {
        return None;
    }
    parts.hour = u8::try_from(hour).ok()?;
    parts.minute = minute;
    parts.second = second;
    parts.microseconds = microseconds;
    Some(parts)
}

fn parse_timestamp(value: &str) -> Option<DateParts> {
    let parts = parse_datetime(value)?;
    let value = (
        parts.year,
        parts.month,
        parts.day,
        parts.hour,
        parts.minute,
        parts.second,
        parts.microseconds,
    );
    ((1970, 1, 1, 0, 0, 1, 0)..=(2038, 1, 19, 3, 14, 7, 499_999))
        .contains(&value)
        .then_some(parts)
}

fn parse_time(value: &str) -> Option<OwnedMySQLValue> {
    let (negative, value) = value
        .strip_prefix('-')
        .map_or((false, value), |value| (true, value));
    let (hours, minutes, seconds, microseconds) = parse_clock(value)?;
    if hours > 838 {
        return None;
    }
    Some(OwnedMySQLValue::Time {
        negative,
        days: u32::from(hours / 24),
        hours: u8::try_from(hours % 24).ok()?,
        minutes,
        seconds,
        microseconds,
    })
}

fn parse_clock(value: &str) -> Option<(u16, u8, u8, u32)> {
    let mut parts = value.split(':');
    let hours = parts.next()?.parse().ok()?;
    let minutes = parts.next()?.parse().ok()?;
    let seconds = parts.next()?;
    if parts.next().is_some() || minutes > 59 {
        return None;
    }
    let (seconds, microseconds) = if let Some((seconds, fraction)) = seconds.split_once('.') {
        if fraction.is_empty()
            || fraction.len() > 6
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        let digits = fraction.len();
        let fraction = fraction.parse::<u32>().ok()?;
        (seconds, fraction.checked_mul(10u32.pow(6 - digits as u32))?)
    } else {
        (seconds, 0)
    };
    let seconds = seconds.parse().ok()?;
    (seconds <= 59).then_some((hours, minutes, seconds, microseconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_temporal_values_with_microseconds() {
        let OwnedMySQLValue::Date { microseconds, .. } = parse_datetime("2024-02-29 23:59:58.0123")
            .unwrap()
            .into_value()
        else {
            unreachable!()
        };
        assert_eq!(microseconds, 12_300);
        assert!(parse_datetime("2023-02-29 00:00:00").is_none());
        assert!(parse_timestamp("1969-12-31 23:59:59").is_none());
        assert!(parse_timestamp("1970-01-01 00:00:01").is_some());
        assert!(parse_timestamp("2038-01-19 03:14:07.500000").is_none());
    }

    #[test]
    fn enforces_mysql_integer_widths() {
        assert!(integer_value(127, MySQLTypeCategory::TinyInt).is_ok());
        assert!(integer_value(128, MySQLTypeCategory::TinyInt).is_err());
        assert_eq!(
            integer_value(255, MySQLTypeCategory::TinyIntUnsigned).unwrap(),
            OwnedMySQLValue::UInt(255)
        );
        assert!(integer_value(-1, MySQLTypeCategory::TinyIntUnsigned).is_err());
    }
}
