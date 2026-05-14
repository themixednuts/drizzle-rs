//! `SqliteValueRow` adapter for [`libsql::Row`].
//!
//! All leaf `FromDrizzleRow` impls live in [`super::sqlite_value`] keyed on
//! the [`super::sqlite_value::SqliteValueRow`] trait. This file is just the
//! one-screen normalizer that converts a `libsql::Value` into a
//! [`super::sqlite_value::SqliteCell`].

use crate::error::DrizzleError;
use crate::row::sqlite_value::{SqliteCell, SqliteValueRow};

/// Convert a 0-based column offset into the `i32` index expected by
/// [`libsql::Row::get_value`], erroring if the offset would truncate or wrap.
#[inline]
fn column_index(offset: usize) -> Result<i32, DrizzleError> {
    i32::try_from(offset).map_err(|_| {
        DrizzleError::ConversionError(format!("column offset {offset} does not fit in i32").into())
    })
}

impl SqliteValueRow for ::libsql::Row {
    fn cell_at(&self, offset: usize) -> Result<SqliteCell, DrizzleError> {
        let val = self
            .get_value(column_index(offset)?)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        Ok(match val {
            ::libsql::Value::Null => SqliteCell::Null,
            ::libsql::Value::Integer(i) => SqliteCell::Integer(i),
            ::libsql::Value::Real(r) => SqliteCell::Real(r),
            ::libsql::Value::Text(s) => SqliteCell::Text(s),
            ::libsql::Value::Blob(b) => SqliteCell::Blob(b),
        })
    }
}
