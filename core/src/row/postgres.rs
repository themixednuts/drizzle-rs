//! Shared `FromDrizzleRow` machinery for Postgres-flavored driver rows.
//!
//! `tokio_postgres::Row` and `postgres::Row` both expose `try_get::<T>(offset)`
//! that delegates to `postgres_types::FromSql<'a>`. Previously the leaf
//! `FromDrizzleRow` impls for every Rust target type were duplicated across
//! the two row types — every feature-gated list (uuid, chrono, serde,
//! rust-decimal, cidr, geo-types, bit-vec) was invoked twice with two
//! macros (`impl_leaf_postgres!` + `impl_option_leaf_postgres!`).
//!
//! The leaf impls cannot be a single blanket keyed on a `PostgresValueRow`
//! bound the way [`SqliteValueRow`](super::sqlite_value::SqliteValueRow) is —
//! a downstream crate could impl both row traits for a single row type, and
//! coherence rejects the resulting `impl FromDrizzleRow<R> for i64` overlap.
//! So instead this module:
//!
//! * declares the [`PostgresValueRow`] trait as a *method-dispatch shim*
//!   (one adapter per row type that centralizes error mapping);
//! * exposes one mega-macro `impl_postgres_value_row!` that, given a
//!   concrete row type, emits every leaf `FromDrizzleRow` impl and the
//!   composite `Option<T: NullProbeRow<R>>` impl;
//! * invokes the mega-macro once per enabled driver.
//!
//! Adding a third postgres-flavored driver becomes: impl `PostgresValueRow`
//! for its row type + `impl_postgres_value_row!(NewRow);`. The full type
//! list lives in one place.

use crate::error::DrizzleError;
use crate::row::{FromDrizzleRow, NullProbeRow};

// Re-export the `FromSql` trait from whichever postgres crate is enabled.
// Both `tokio_postgres::types::FromSql` and `postgres::types::FromSql`
// resolve to the same `postgres_types::FromSql<'a>` trait at link time, so
// only one needs to be in scope.
#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
pub use ::postgres::types::FromSql;
#[cfg(feature = "tokio-postgres")]
pub use ::tokio_postgres::types::FromSql;

/// Implemented by Postgres-flavored row types whose cells are decoded through
/// `postgres_types::FromSql`. Drivers supply a one-method adapter that
/// forwards to their native `try_get`; `impl_postgres_value_row!` then emits
/// every leaf [`FromDrizzleRow`] impl for that row type.
pub trait PostgresValueRow {
    /// Fetch the column at `offset` and decode it through `FromSql`,
    /// normalising the driver-specific error type to [`DrizzleError`].
    fn try_get_from_sql<'a, T>(&'a self, offset: usize) -> Result<T, DrizzleError>
    where
        T: FromSql<'a>;
}

// =============================================================================
// Leaf-impl macros — bind to a concrete row type to avoid the
// blanket-over-R coherence trap. Body is uniform: defer to the trait method,
// which the driver supplies.
// =============================================================================

macro_rules! postgres_leaf_impls {
    ($row_ty:ty; $($ty:ty),* $(,)?) => { $(
        impl FromDrizzleRow<$row_ty> for $ty {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &$row_ty, offset: usize) -> Result<Self, DrizzleError> {
                <$row_ty as PostgresValueRow>::try_get_from_sql(row, offset)
            }
        }
        impl FromDrizzleRow<$row_ty> for Option<$ty> {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &$row_ty, offset: usize) -> Result<Self, DrizzleError> {
                <$row_ty as PostgresValueRow>::try_get_from_sql(row, offset)
            }
        }
    )* };
}

/// Emit every `FromDrizzleRow` leaf impl and the composite
/// `Option<T: NullProbeRow<R>>` impl for `$row_ty`. Invoked once per enabled
/// postgres-flavored driver.
macro_rules! impl_postgres_value_row {
    ($row_ty:ty) => {
        postgres_leaf_impls!(
            $row_ty;
            i8, i16, i32, i64, f32, f64, bool, String,
            Vec<u8>,
            Vec<i16>, Vec<i32>, Vec<i64>, Vec<f32>, Vec<f64>,
            Vec<bool>, Vec<String>,
        );

        #[cfg(feature = "uuid")]
        postgres_leaf_impls!($row_ty; uuid::Uuid, Vec<uuid::Uuid>);

        #[cfg(feature = "chrono")]
        postgres_leaf_impls!(
            $row_ty;
            chrono::NaiveDate, chrono::NaiveTime, chrono::NaiveDateTime,
            chrono::DateTime<chrono::Utc>,
            Vec<chrono::NaiveDate>, Vec<chrono::NaiveTime>,
            Vec<chrono::NaiveDateTime>, Vec<chrono::DateTime<chrono::Utc>>,
        );

        #[cfg(feature = "serde")]
        postgres_leaf_impls!($row_ty; serde_json::Value, Vec<serde_json::Value>);

        #[cfg(feature = "rust-decimal")]
        postgres_leaf_impls!($row_ty; rust_decimal::Decimal);

        #[cfg(feature = "cidr")]
        postgres_leaf_impls!($row_ty; cidr::IpInet, cidr::IpCidr);

        #[cfg(feature = "geo-types")]
        postgres_leaf_impls!(
            $row_ty;
            geo_types::Point<f64>,
            geo_types::LineString<f64>,
            geo_types::Rect<f64>,
        );

        #[cfg(feature = "bit-vec")]
        postgres_leaf_impls!($row_ty; bit_vec::BitVec);

        // Composite multi-column Option<T> — falls through to the driver's
        // null probe and recurses into T::from_row_at on a non-NULL leading
        // column. Bound to a concrete row type to coexist with the
        // `Option<ConcreteT>` leaves above.
        impl<T> FromDrizzleRow<$row_ty> for Option<T>
        where
            T: NullProbeRow<$row_ty>,
        {
            const COLUMN_COUNT: usize = T::COLUMN_COUNT;
            fn from_row_at(row: &$row_ty, offset: usize) -> Result<Self, DrizzleError> {
                if T::is_null_at(row, offset)? {
                    return Ok(None);
                }
                T::from_row_at(row, offset).map(Some)
            }
        }
    };
}

// =============================================================================
// Driver adapters — one trait impl + one macro invocation each.
// =============================================================================

#[cfg(feature = "tokio-postgres")]
impl PostgresValueRow for ::tokio_postgres::Row {
    fn try_get_from_sql<'a, T>(&'a self, offset: usize) -> Result<T, DrizzleError>
    where
        T: FromSql<'a>,
    {
        self.try_get(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(feature = "tokio-postgres")]
impl_postgres_value_row!(::tokio_postgres::Row);

#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
impl PostgresValueRow for ::postgres::Row {
    fn try_get_from_sql<'a, T>(&'a self, offset: usize) -> Result<T, DrizzleError>
    where
        T: FromSql<'a>,
    {
        self.try_get(offset)
            .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))
    }
}

#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
impl_postgres_value_row!(::postgres::Row);
