//! `FromDrizzleRow` leaf impls for [`turso::Row`].

use crate::error::DrizzleError;
use crate::row::FromDrizzleRow;

// -- Integer types --

macro_rules! impl_leaf_turso_int {
    ($($ty:ty),*) => { $(
        impl FromDrizzleRow<::turso::Row> for $ty {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
                let val = row.get_value(offset)
                    .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
                if val.is_null() {
                    return Err(DrizzleError::ConversionError("unexpected NULL for integer".into()));
                }
                if let Some(i) = val.as_integer() {
                    (*i).try_into()
                        .map_err(|e: core::num::TryFromIntError| DrizzleError::ConversionError(e.to_string().into()))
                } else {
                    Err(DrizzleError::ConversionError("expected integer value".into()))
                }
            }
        }
    )* }
}

impl_leaf_turso_int!(i8, i16, i32, isize, u8, u16, u32, u64, usize);

// i64 doesn't need try_into (identity conversion)
impl FromDrizzleRow<::turso::Row> for i64 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if val.is_null() {
            return Err(DrizzleError::ConversionError(
                "unexpected NULL for integer".into(),
            ));
        }
        if let Some(i) = val.as_integer() {
            Ok(*i)
        } else {
            Err(DrizzleError::ConversionError(
                "expected integer value".into(),
            ))
        }
    }
}

// -- Float types --

impl FromDrizzleRow<::turso::Row> for f64 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if val.is_null() {
            return Err(DrizzleError::ConversionError(
                "unexpected NULL for float".into(),
            ));
        }
        if let Some(r) = val.as_real() {
            Ok(*r)
        } else if let Some(i) = val.as_integer() {
            Ok(*i as f64)
        } else {
            Err(DrizzleError::ConversionError("expected real value".into()))
        }
    }
}

impl FromDrizzleRow<::turso::Row> for f32 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        Ok(f64::from_row_at(row, offset)? as f32)
    }
}

// -- Bool --

impl FromDrizzleRow<::turso::Row> for bool {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if val.is_null() {
            return Err(DrizzleError::ConversionError(
                "unexpected NULL for bool".into(),
            ));
        }
        if let Some(i) = val.as_integer() {
            Ok(*i != 0)
        } else {
            Err(DrizzleError::ConversionError(
                "expected integer for bool".into(),
            ))
        }
    }
}

// -- String --

impl FromDrizzleRow<::turso::Row> for String {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if val.is_null() {
            return Err(DrizzleError::ConversionError(
                "unexpected NULL for string".into(),
            ));
        }
        if let Some(s) = val.as_text() {
            Ok(s.to_string())
        } else {
            Err(DrizzleError::ConversionError("expected text value".into()))
        }
    }
}

// -- Vec<u8> --

impl FromDrizzleRow<::turso::Row> for Vec<u8> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if val.is_null() {
            return Err(DrizzleError::ConversionError(
                "unexpected NULL for blob".into(),
            ));
        }
        if let Some(b) = val.as_blob() {
            Ok(b.to_vec())
        } else {
            Err(DrizzleError::ConversionError("expected blob value".into()))
        }
    }
}

// -- Option<T>: NULL-aware wrapper --

impl<T: FromDrizzleRow<::turso::Row>> FromDrizzleRow<::turso::Row> for Option<T> {
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if val.is_null() {
            Ok(None)
        } else {
            T::from_row_at(row, offset).map(Some)
        }
    }
}

// -- Feature-gated types --

#[cfg(feature = "uuid")]
impl FromDrizzleRow<::turso::Row> for uuid::Uuid {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if let Some(s) = val.as_text() {
            uuid::Uuid::parse_str(s).map_err(Into::into)
        } else if let Some(b) = val.as_blob() {
            uuid::Uuid::from_slice(b)
                .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))
        } else {
            Err(DrizzleError::ConversionError(
                "expected TEXT or BLOB for UUID".into(),
            ))
        }
    }
}

#[cfg(feature = "chrono")]
impl FromDrizzleRow<::turso::Row> for chrono::NaiveDate {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl FromDrizzleRow<::turso::Row> for chrono::NaiveTime {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl FromDrizzleRow<::turso::Row> for chrono::NaiveDateTime {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl FromDrizzleRow<::turso::Row> for chrono::DateTime<chrono::Utc> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
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
impl FromDrizzleRow<::turso::Row> for serde_json::Value {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::turso::Row, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        serde_json::from_str(&s).map_err(Into::into)
    }
}
