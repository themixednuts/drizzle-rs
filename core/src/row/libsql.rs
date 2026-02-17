//! `FromDrizzleRow` leaf impls for [`libsql::Row`].

use crate::error::DrizzleError;
use crate::row::FromDrizzleRow;

// -- Integer types --

macro_rules! impl_leaf_libsql_int {
    ($($ty:ty),*) => { $(
        impl FromDrizzleRow<::libsql::Row> for $ty {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
                let val = row.get_value(offset as i32)
                    .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
                match val {
                    ::libsql::Value::Integer(i) => i.try_into()
                        .map_err(|e: core::num::TryFromIntError| DrizzleError::ConversionError(e.to_string().into())),
                    ::libsql::Value::Null => Err(DrizzleError::ConversionError("unexpected NULL for integer".into())),
                    _ => Err(DrizzleError::ConversionError("expected integer value".into())),
                }
            }
        }
    )* }
}

impl_leaf_libsql_int!(i8, i16, i32, isize, u8, u16, u32, u64, usize);

// i64 doesn't need try_into (identity conversion produces Infallible, not TryFromIntError)
impl FromDrizzleRow<::libsql::Row> for i64 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset as i32)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        match val {
            ::libsql::Value::Integer(i) => Ok(i),
            ::libsql::Value::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for integer".into(),
            )),
            _ => Err(DrizzleError::ConversionError(
                "expected integer value".into(),
            )),
        }
    }
}

// -- Float types --

impl FromDrizzleRow<::libsql::Row> for f64 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset as i32)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        match val {
            ::libsql::Value::Real(r) => Ok(r),
            ::libsql::Value::Integer(i) => Ok(i as f64),
            ::libsql::Value::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for float".into(),
            )),
            _ => Err(DrizzleError::ConversionError("expected real value".into())),
        }
    }
}

impl FromDrizzleRow<::libsql::Row> for f32 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        Ok(f64::from_row_at(row, offset)? as f32)
    }
}

// -- Bool --

impl FromDrizzleRow<::libsql::Row> for bool {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset as i32)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        match val {
            ::libsql::Value::Integer(i) => Ok(i != 0),
            ::libsql::Value::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for bool".into(),
            )),
            _ => Err(DrizzleError::ConversionError(
                "expected integer for bool".into(),
            )),
        }
    }
}

// -- String --

impl FromDrizzleRow<::libsql::Row> for String {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset as i32)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        match val {
            ::libsql::Value::Text(s) => Ok(s),
            ::libsql::Value::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for string".into(),
            )),
            _ => Err(DrizzleError::ConversionError("expected text value".into())),
        }
    }
}

// -- Vec<u8> --

impl FromDrizzleRow<::libsql::Row> for Vec<u8> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset as i32)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        match val {
            ::libsql::Value::Blob(b) => Ok(b),
            ::libsql::Value::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for blob".into(),
            )),
            _ => Err(DrizzleError::ConversionError("expected blob value".into())),
        }
    }
}

// -- Option<T>: NULL-aware wrapper --

impl<T: FromDrizzleRow<::libsql::Row>> FromDrizzleRow<::libsql::Row> for Option<T> {
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset as i32)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if matches!(val, ::libsql::Value::Null) {
            Ok(None)
        } else {
            T::from_row_at(row, offset).map(Some)
        }
    }
}

// -- Feature-gated types --

#[cfg(feature = "uuid")]
impl FromDrizzleRow<::libsql::Row> for uuid::Uuid {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset as i32)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        match val {
            ::libsql::Value::Text(s) => uuid::Uuid::parse_str(&s).map_err(Into::into),
            ::libsql::Value::Blob(b) => uuid::Uuid::from_slice(&b)
                .map_err(|e| DrizzleError::ConversionError(e.to_string().into())),
            _ => Err(DrizzleError::ConversionError(
                "expected TEXT or BLOB for UUID".into(),
            )),
        }
    }
}

#[cfg(feature = "chrono")]
impl FromDrizzleRow<::libsql::Row> for chrono::NaiveDate {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl FromDrizzleRow<::libsql::Row> for chrono::NaiveTime {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl FromDrizzleRow<::libsql::Row> for chrono::NaiveDateTime {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl FromDrizzleRow<::libsql::Row> for chrono::DateTime<chrono::Utc> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        let ndt: chrono::NaiveDateTime = s
            .parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))?;
        Ok(chrono::DateTime::from_naive_utc_and_offset(
            ndt,
            chrono::Utc,
        ))
    }
}

#[cfg(feature = "serde")]
impl FromDrizzleRow<::libsql::Row> for serde_json::Value {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        serde_json::from_str(&s).map_err(Into::into)
    }
}
