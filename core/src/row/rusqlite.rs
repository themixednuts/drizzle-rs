//! `FromDrizzleRow` leaf impls for [`rusqlite::Row`].

use crate::error::DrizzleError;
use crate::row::FromDrizzleRow;

// -- Types with native rusqlite::types::FromSql --

macro_rules! impl_leaf_rusqlite_via_get {
    ($($ty:ty),*) => { $(
        impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for $ty {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
                Ok(row.get(offset)?)
            }
        }
    )* }
}

impl_leaf_rusqlite_via_get!(
    i8,
    i16,
    i32,
    i64,
    isize,
    u8,
    u16,
    u32,
    f64,
    bool,
    String,
    Vec<u8>
);

// -- Types without native FromSql: convert from a supported type --

impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for f32 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        Ok(row.get::<_, f64>(offset)? as f32)
    }
}

impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for u64 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        Ok(row.get::<_, i64>(offset)?.try_into()?)
    }
}

impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for usize {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        Ok(row.get::<_, i64>(offset)?.try_into()?)
    }
}

// -- Option<T>: NULL-aware wrapper --

impl<'r, T: FromDrizzleRow<::rusqlite::Row<'r>>> FromDrizzleRow<::rusqlite::Row<'r>> for Option<T> {
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        let ref_val = row.get_ref(offset)?;
        if matches!(ref_val, ::rusqlite::types::ValueRef::Null) {
            Ok(None)
        } else {
            T::from_row_at(row, offset).map(Some)
        }
    }
}

// -- Feature-gated types --

#[cfg(feature = "uuid")]
impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for uuid::Uuid {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        // rusqlite stores UUIDs as TEXT or BLOB; try text first
        let ref_val = row.get_ref(offset)?;
        match ref_val {
            ::rusqlite::types::ValueRef::Text(bytes) => {
                let s = core::str::from_utf8(bytes)
                    .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
                uuid::Uuid::parse_str(s).map_err(Into::into)
            }
            ::rusqlite::types::ValueRef::Blob(bytes) => uuid::Uuid::from_slice(bytes)
                .map_err(|e| DrizzleError::ConversionError(e.to_string().into())),
            _ => Err(DrizzleError::ConversionError(
                "expected TEXT or BLOB for UUID".into(),
            )),
        }
    }
}

#[cfg(feature = "chrono")]
impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for chrono::NaiveDate {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        let s: String = row.get(offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for chrono::NaiveTime {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        let s: String = row.get(offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for chrono::NaiveDateTime {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        let s: String = row.get(offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for chrono::DateTime<chrono::Utc> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        // rusqlite stores timestamps as strings; parse via NaiveDateTime then assume UTC
        let s: String = row.get(offset)?;
        let ndt = s
            .parse::<chrono::NaiveDateTime>()
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        Ok(chrono::DateTime::from_naive_utc_and_offset(
            ndt,
            chrono::Utc,
        ))
    }
}

#[cfg(feature = "serde")]
impl<'r> FromDrizzleRow<::rusqlite::Row<'r>> for serde_json::Value {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &::rusqlite::Row<'r>, offset: usize) -> Result<Self, DrizzleError> {
        let s: String = row.get(offset)?;
        serde_json::from_str(&s).map_err(Into::into)
    }
}
