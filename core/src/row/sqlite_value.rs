//! Shared `FromDrizzleRow` machinery for SQLite-flavored driver rows.
//!
//! `rusqlite::Row`, `libsql::Row`, and `turso::Row` all expose a single
//! per-cell fetch that returns a tagged union of integer / real / text /
//! blob / null. The leaf `FromDrizzleRow` impls for each driver were
//! near-clones of the same match-on-variant pattern, differing only in how
//! the fetch is spelled.
//!
//! This module captures that pattern as the [`SqliteValueRow`] trait. Each
//! driver supplies one tiny adapter that normalizes its native cell into a
//! [`SqliteCell`]; the leaf [`FromDrizzleRow`] impls for every Rust target
//! type (`i64`, `f64`, `String`, `Vec<u8>`, `bool`, `uuid::Uuid`, chrono /
//! serde types, `Option<T>`) live here once as blanket impls keyed on
//! `R: SqliteValueRow`.
//!
//! ## NULL probes
//!
//! `Option<T>::from_row_at` only needs to know whether a cell is NULL — it
//! shouldn't pay the cost of materialising a large `Text`/`Blob` value into
//! an owned [`SqliteCell`] just to throw it away. The trait therefore
//! exposes a dedicated [`SqliteValueRow::is_null_at`] method with a
//! `cell_at`-based default impl. Drivers that can probe NULL without
//! allocating (e.g. `rusqlite::Row::get_ref`) override it.

use crate::error::DrizzleError;
use crate::row::FromDrizzleRow;

/// SQLite-flavored cell value. The union of the four storage classes plus NULL,
/// matching what `libsql::Value` and `turso::Value` already expose.
#[derive(Debug, Clone)]
pub enum SqliteCell {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl SqliteCell {
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// Implemented by SQLite-flavored row types whose cells live in a tagged
/// union (`rusqlite::ValueRef`, `libsql::Value`, `turso::Value`). Drivers
/// normalize one cell into a [`SqliteCell`]; the shared blanket impls below
/// take care of every target type.
pub trait SqliteValueRow {
    /// Fetch the column at `offset` and normalize it into a [`SqliteCell`].
    /// Errors should reflect driver-side I/O / range failures — `Ok(Null)`
    /// is the standard "NULL was here" outcome, not an error.
    fn cell_at(&self, offset: usize) -> Result<SqliteCell, DrizzleError>;

    /// Return `true` if the column at `offset` is NULL.
    ///
    /// The default implementation materialises the cell and inspects its
    /// tag. Drivers that can probe NULL without allocating (e.g.
    /// `rusqlite::Row::get_ref`) should override this — the `Option<T>`
    /// blanket calls `is_null_at` on every fetch, so avoiding the
    /// materialisation matters for large `Text` / `Blob` columns.
    #[inline]
    fn is_null_at(&self, offset: usize) -> Result<bool, DrizzleError> {
        Ok(self.cell_at(offset)?.is_null())
    }
}

// =============================================================================
// Integer types
// =============================================================================

/// Produce the IEEE 754 `f64` representation of an `i64`, matching `i as f64`
/// semantics without using a precision-losing `as` cast.
///
/// Splits `i` into a sign-extended high `i32` half and an unsigned low `u32`
/// half, both converted via the exact [`From`] trait, then recombines with a
/// `2^32` multiply. The result is identical to `i as f64` for all `i64`
/// inputs (inexact beyond `|i| > 2^53` in the same way the direct cast is).
#[inline]
fn i64_to_f64(i: i64) -> f64 {
    let high = i32::try_from(i >> 32).expect("sign-extended high word fits in i32");
    let low = u32::try_from(i & 0xFFFF_FFFF).expect("masked low word fits in u32");
    f64::from(high) * 4_294_967_296.0_f64 + f64::from(low)
}

macro_rules! sqlite_value_int_impl {
    ($($ty:ty),*) => { $(
        impl<R: SqliteValueRow> FromDrizzleRow<R> for $ty {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
                match row.cell_at(offset)? {
                    SqliteCell::Integer(i) => i.try_into().map_err(
                        |e: core::num::TryFromIntError| {
                            DrizzleError::ConversionError(e.to_string().into())
                        },
                    ),
                    SqliteCell::Null => Err(DrizzleError::ConversionError(
                        "unexpected NULL for integer".into(),
                    )),
                    _ => Err(DrizzleError::ConversionError(
                        "expected integer value".into(),
                    )),
                }
            }
        }
    )* }
}

sqlite_value_int_impl!(i8, i16, i32, isize, u8, u16, u32, u64, usize);

// `i64` is the identity conversion — skip the `try_into` indirection so the
// fast path doesn't even mention `TryFromIntError`.
impl<R: SqliteValueRow> FromDrizzleRow<R> for i64 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        match row.cell_at(offset)? {
            SqliteCell::Integer(i) => Ok(i),
            SqliteCell::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for integer".into(),
            )),
            _ => Err(DrizzleError::ConversionError(
                "expected integer value".into(),
            )),
        }
    }
}

// =============================================================================
// Float types
// =============================================================================

impl<R: SqliteValueRow> FromDrizzleRow<R> for f64 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        match row.cell_at(offset)? {
            SqliteCell::Real(r) => Ok(r),
            // SQLite's NUMERIC affinity allows an integer to come back from a
            // column declared REAL; preserve the existing libsql behavior of
            // accepting that and round-tripping via the exact `i64_to_f64`
            // helper. (turso used a decimal-string parse for the same idea —
            // this is the more correct path.)
            SqliteCell::Integer(i) => Ok(i64_to_f64(i)),
            SqliteCell::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for float".into(),
            )),
            _ => Err(DrizzleError::ConversionError("expected real value".into())),
        }
    }
}

impl<R: SqliteValueRow> FromDrizzleRow<R> for f32 {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        let v = f64::from_row_at(row, offset)?;
        // Decimal-string round-trip matches IEEE-754 round-to-nearest
        // semantics and avoids the lossy `as` cast.
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

// =============================================================================
// Bool, String, Vec<u8>
// =============================================================================

impl<R: SqliteValueRow> FromDrizzleRow<R> for bool {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        match row.cell_at(offset)? {
            SqliteCell::Integer(i) => Ok(i != 0),
            SqliteCell::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for bool".into(),
            )),
            _ => Err(DrizzleError::ConversionError(
                "expected integer for bool".into(),
            )),
        }
    }
}

impl<R: SqliteValueRow> FromDrizzleRow<R> for String {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        match row.cell_at(offset)? {
            SqliteCell::Text(s) => Ok(s),
            SqliteCell::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for string".into(),
            )),
            _ => Err(DrizzleError::ConversionError("expected text value".into())),
        }
    }
}

impl<R: SqliteValueRow> FromDrizzleRow<R> for Vec<u8> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        match row.cell_at(offset)? {
            SqliteCell::Blob(b) => Ok(b),
            SqliteCell::Null => Err(DrizzleError::ConversionError(
                "unexpected NULL for blob".into(),
            )),
            _ => Err(DrizzleError::ConversionError("expected blob value".into())),
        }
    }
}

// =============================================================================
// Option<T>: NULL-aware wrapper
// =============================================================================

impl<R: SqliteValueRow, T> FromDrizzleRow<R> for Option<T>
where
    T: FromDrizzleRow<R>,
{
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        if row.is_null_at(offset)? {
            Ok(None)
        } else {
            T::from_row_at(row, offset).map(Some)
        }
    }
}

// =============================================================================
// Feature-gated types
// =============================================================================

#[cfg(feature = "uuid")]
impl<R: SqliteValueRow> FromDrizzleRow<R> for uuid::Uuid {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        match row.cell_at(offset)? {
            SqliteCell::Text(s) => Self::parse_str(&s).map_err(Into::into),
            SqliteCell::Blob(b) => Self::from_slice(&b)
                .map_err(|e| DrizzleError::ConversionError(e.to_string().into())),
            _ => Err(DrizzleError::ConversionError(
                "expected TEXT or BLOB for UUID".into(),
            )),
        }
    }
}

#[cfg(feature = "chrono")]
impl<R: SqliteValueRow> FromDrizzleRow<R> for chrono::NaiveDate {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl<R: SqliteValueRow> FromDrizzleRow<R> for chrono::NaiveTime {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl<R: SqliteValueRow> FromDrizzleRow<R> for chrono::NaiveDateTime {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        s.parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "chrono")]
impl<R: SqliteValueRow> FromDrizzleRow<R> for chrono::DateTime<chrono::Utc> {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        let ndt: chrono::NaiveDateTime = s
            .parse()
            .map_err(|e: chrono::ParseError| DrizzleError::ConversionError(e.to_string().into()))?;
        Ok(Self::from_naive_utc_and_offset(ndt, chrono::Utc))
    }
}

#[cfg(feature = "serde")]
impl<R: SqliteValueRow> FromDrizzleRow<R> for serde_json::Value {
    const COLUMN_COUNT: usize = 1;
    fn from_row_at(row: &R, offset: usize) -> Result<Self, DrizzleError> {
        let s = String::from_row_at(row, offset)?;
        serde_json::from_str(&s).map_err(Into::into)
    }
}
