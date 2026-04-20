//! `FromDrizzleRow` leaf impls for [`libsql::Row`].

use crate::error::DrizzleError;
use crate::row::FromDrizzleRow;

/// Convert a 0-based column offset into the `i32` index expected by
/// [`libsql::Row::get_value`], erroring if the offset would truncate or wrap.
#[inline]
fn column_index(offset: usize) -> Result<i32, DrizzleError> {
    i32::try_from(offset).map_err(|_| {
        DrizzleError::ConversionError(format!("column offset {offset} does not fit in i32").into())
    })
}

/// Produce the IEEE 754 `f64` representation of an `i64`, matching `i as f64`
/// semantics without using a precision-losing `as` cast.
///
/// Splits `i` into a sign-extended high `i32` half and an unsigned low `u32`
/// half, both converted via the exact [`From`] trait, then recombines with a
/// `2^32` multiply. The result is identical to `i as f64` for all `i64`
/// inputs (inexact beyond `|i| > 2^53` in the same way the direct cast is).
#[inline]
fn i64_to_f64(i: i64) -> f64 {
    // `i >> 32` for `i64` sign-extends into the range `[i32::MIN, i32::MAX]`.
    let high = i32::try_from(i >> 32).expect("sign-extended high word fits in i32");
    // `i & 0xFFFF_FFFF` masks to the low 32 bits, always in `[0, u32::MAX]`.
    let low = u32::try_from(i & 0xFFFF_FFFF).expect("masked low word fits in u32");
    f64::from(high) * 4_294_967_296.0_f64 + f64::from(low)
}

// -- Integer types --

macro_rules! impl_leaf_libsql_int {
    ($($ty:ty),*) => { $(
        impl FromDrizzleRow<::libsql::Row> for $ty {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
                let val = row.get_value(column_index(offset)?)
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
            .get_value(column_index(offset)?)
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
            .get_value(column_index(offset)?)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        match val {
            ::libsql::Value::Real(r) => Ok(r),
            ::libsql::Value::Integer(i) => Ok(i64_to_f64(i)),
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
        let v = f64::from_row_at(row, offset)?;
        // Decimal-string round-trip matches IEEE-754 round-to-nearest semantics
        // and avoids the lossy `as` cast.
        let f: Self = format!("{v}")
            .parse()
            .map_err(|e: core::num::ParseFloatError| {
                DrizzleError::ConversionError(e.to_string().into())
            })?;
        if v.is_finite() && !f.is_finite() {
            return Err(DrizzleError::ConversionError(
                format!("f64 value {v} overflows f32").into(),
            ));
        }
        Ok(f)
    }
}

// -- Bool --

impl FromDrizzleRow<::libsql::Row> for bool {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::libsql::Row, offset: usize) -> Result<Self, DrizzleError> {
        let val = row
            .get_value(column_index(offset)?)
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
            .get_value(column_index(offset)?)
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
            .get_value(column_index(offset)?)
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
            .get_value(column_index(offset)?)
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
            .get_value(column_index(offset)?)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        match val {
            ::libsql::Value::Text(s) => Self::parse_str(&s).map_err(Into::into),
            ::libsql::Value::Blob(b) => Self::from_slice(&b)
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
        Ok(Self::from_naive_utc_and_offset(ndt, chrono::Utc))
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
