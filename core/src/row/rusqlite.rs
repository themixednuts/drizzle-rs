//! `SqliteValueRow` adapter for [`rusqlite::Row`].
//!
//! Every leaf `FromDrizzleRow` impl lives in
//! [`super::sqlite_value`](crate::row::sqlite_value); this file supplies
//! only the one trait impl that maps rusqlite's `ValueRef` into a
//! [`SqliteCell`] and overrides `is_null_at` to use `Row::get_ref` for a
//! zero-allocation NULL probe.

use crate::error::DrizzleError;
use crate::row::sqlite_value::{SqliteCell, SqliteValueRow};

impl SqliteValueRow for ::rusqlite::Row<'_> {
    fn cell_at(&self, offset: usize) -> Result<SqliteCell, DrizzleError> {
        let value = self
            .get_ref(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        Ok(match value {
            ::rusqlite::types::ValueRef::Null => SqliteCell::Null,
            ::rusqlite::types::ValueRef::Integer(i) => SqliteCell::Integer(i),
            ::rusqlite::types::ValueRef::Real(r) => SqliteCell::Real(r),
            ::rusqlite::types::ValueRef::Text(bytes) => {
                let s = core::str::from_utf8(bytes)
                    .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?
                    .to_owned();
                SqliteCell::Text(s)
            }
            ::rusqlite::types::ValueRef::Blob(bytes) => SqliteCell::Blob(bytes.to_vec()),
        })
    }

    /// Zero-allocation NULL probe — inspects the cell tag without
    /// materialising `Text` / `Blob` payloads. The `Option<T>` blanket
    /// hits this on every fetch.
    #[inline]
    fn is_null_at(&self, offset: usize) -> Result<bool, DrizzleError> {
        let value = self
            .get_ref(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))?;
        Ok(matches!(value, ::rusqlite::types::ValueRef::Null))
    }
}
