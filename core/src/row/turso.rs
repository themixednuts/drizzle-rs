//! `SqliteValueRow` adapter for [`turso::Row`].
//!
//! All leaf `FromDrizzleRow` impls live in [`super::sqlite_value`] keyed on
//! the [`super::sqlite_value::SqliteValueRow`] trait. This file is just the
//! one-screen normalizer that converts a `turso::Value` into a
//! [`super::sqlite_value::SqliteCell`].

use crate::error::DrizzleError;
use crate::row::sqlite_value::{SqliteCell, SqliteValueRow};

impl SqliteValueRow for ::turso::Row {
    fn cell_at(&self, offset: usize) -> Result<SqliteCell, DrizzleError> {
        let val = self
            .get_value(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        if val.is_null() {
            return Ok(SqliteCell::Null);
        }
        if let Some(i) = val.as_integer() {
            return Ok(SqliteCell::Integer(*i));
        }
        if let Some(r) = val.as_real() {
            return Ok(SqliteCell::Real(*r));
        }
        if let Some(s) = val.as_text() {
            return Ok(SqliteCell::Text(s.to_string()));
        }
        if let Some(b) = val.as_blob() {
            return Ok(SqliteCell::Blob(b.to_vec()));
        }
        Err(DrizzleError::ConversionError(
            "turso::Value variant outside SQLite storage classes".into(),
        ))
    }
}
