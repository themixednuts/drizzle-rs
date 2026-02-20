//! `FromDrizzleRow` leaf impls for PostgreSQL driver rows.
//!
//! When `tokio-postgres` is enabled, we impl for `tokio_postgres::Row`.
//! When only `postgres-sync` is enabled, we impl for `postgres::Row`.
//! Both share the same underlying `FromSql` trait from `postgres-types`,
//! so the generated code is identical.

use crate::error::DrizzleError;
use crate::row::{FromDrizzleRow, NullProbeRow};

// Determine which Row type to use. tokio-postgres takes priority
// because when both are enabled, postgres re-exports tokio-postgres types.

macro_rules! impl_leaf_postgres {
    ($row_ty:ty; $($ty:ty),*) => { $(
        impl FromDrizzleRow<$row_ty> for $ty {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &$row_ty, offset: usize) -> Result<Self, DrizzleError> {
                row.try_get(offset).map_err(|e| DrizzleError::ConversionError(e.to_string().into()))
            }
        }
    )* }
}

/// Concrete `Option<T>` impls for leaf types using native postgres `FromSql`
/// for `Option<T>`, which correctly returns `Ok(None)` for NULL and `Err`
/// for conversion failures.
macro_rules! impl_option_leaf_postgres {
    ($row_ty:ty; $($ty:ty),*) => { $(
        impl FromDrizzleRow<$row_ty> for Option<$ty> {
            const COLUMN_COUNT: usize = 1;
            fn from_row_at(row: &$row_ty, offset: usize) -> Result<Self, DrizzleError> {
                row.try_get(offset)
                    .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))
            }
        }
    )* }
}

// -- tokio-postgres impls --

#[cfg(feature = "tokio-postgres")]
mod tokio_pg {
    use super::*;

    impl_leaf_postgres!(
        ::tokio_postgres::Row;
        i8,
        i16,
        i32,
        i64,
        f32,
        f64,
        bool,
        String,
        Vec<u8>,
        Vec<i16>,
        Vec<i32>,
        Vec<i64>,
        Vec<f32>,
        Vec<f64>,
        Vec<bool>,
        Vec<String>
    );

    impl_option_leaf_postgres!(
        ::tokio_postgres::Row;
        i8,
        i16,
        i32,
        i64,
        f32,
        f64,
        bool,
        String,
        Vec<u8>,
        Vec<i16>,
        Vec<i32>,
        Vec<i64>,
        Vec<f32>,
        Vec<f64>,
        Vec<bool>,
        Vec<String>
    );

    #[cfg(feature = "uuid")]
    impl_leaf_postgres!(::tokio_postgres::Row; uuid::Uuid);
    #[cfg(feature = "uuid")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; uuid::Uuid);
    #[cfg(feature = "uuid")]
    impl_leaf_postgres!(::tokio_postgres::Row; Vec<uuid::Uuid>);
    #[cfg(feature = "uuid")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; Vec<uuid::Uuid>);

    #[cfg(feature = "chrono")]
    impl_leaf_postgres!(::tokio_postgres::Row; chrono::NaiveDate, chrono::NaiveTime, chrono::NaiveDateTime, chrono::DateTime<chrono::Utc>);
    #[cfg(feature = "chrono")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; chrono::NaiveDate, chrono::NaiveTime, chrono::NaiveDateTime, chrono::DateTime<chrono::Utc>);
    #[cfg(feature = "chrono")]
    impl_leaf_postgres!(::tokio_postgres::Row; Vec<chrono::NaiveDate>, Vec<chrono::NaiveTime>, Vec<chrono::NaiveDateTime>, Vec<chrono::DateTime<chrono::Utc>>);
    #[cfg(feature = "chrono")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; Vec<chrono::NaiveDate>, Vec<chrono::NaiveTime>, Vec<chrono::NaiveDateTime>, Vec<chrono::DateTime<chrono::Utc>>);

    #[cfg(feature = "serde")]
    impl_leaf_postgres!(::tokio_postgres::Row; serde_json::Value);
    #[cfg(feature = "serde")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; serde_json::Value);
    #[cfg(feature = "serde")]
    impl_leaf_postgres!(::tokio_postgres::Row; Vec<serde_json::Value>);
    #[cfg(feature = "serde")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; Vec<serde_json::Value>);

    #[cfg(feature = "rust-decimal")]
    impl_leaf_postgres!(::tokio_postgres::Row; rust_decimal::Decimal);
    #[cfg(feature = "rust-decimal")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; rust_decimal::Decimal);

    #[cfg(feature = "cidr")]
    impl_leaf_postgres!(::tokio_postgres::Row; cidr::IpInet, cidr::IpCidr);
    #[cfg(feature = "cidr")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; cidr::IpInet, cidr::IpCidr);

    #[cfg(feature = "geo-types")]
    impl_leaf_postgres!(::tokio_postgres::Row; geo_types::Point<f64>, geo_types::LineString<f64>, geo_types::Rect<f64>);
    #[cfg(feature = "geo-types")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; geo_types::Point<f64>, geo_types::LineString<f64>, geo_types::Rect<f64>);

    #[cfg(feature = "bit-vec")]
    impl_leaf_postgres!(::tokio_postgres::Row; bit_vec::BitVec);
    #[cfg(feature = "bit-vec")]
    impl_option_leaf_postgres!(::tokio_postgres::Row; bit_vec::BitVec);

    // Composite (multi-column) Option<T> via NullProbeRow
    impl<T: NullProbeRow<::tokio_postgres::Row>> FromDrizzleRow<::tokio_postgres::Row> for Option<T> {
        const COLUMN_COUNT: usize = T::COLUMN_COUNT;

        fn from_row_at(row: &::tokio_postgres::Row, offset: usize) -> Result<Self, DrizzleError> {
            if T::is_null_at(row, offset)? {
                return Ok(None);
            }
            T::from_row_at(row, offset).map(Some)
        }
    }
}

// -- postgres-sync impls (only when tokio-postgres is NOT enabled) --

#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
mod sync_pg {
    use super::*;

    impl_leaf_postgres!(
        ::postgres::Row;
        i8,
        i16,
        i32,
        i64,
        f32,
        f64,
        bool,
        String,
        Vec<u8>,
        Vec<i16>,
        Vec<i32>,
        Vec<i64>,
        Vec<f32>,
        Vec<f64>,
        Vec<bool>,
        Vec<String>
    );

    impl_option_leaf_postgres!(
        ::postgres::Row;
        i8,
        i16,
        i32,
        i64,
        f32,
        f64,
        bool,
        String,
        Vec<u8>,
        Vec<i16>,
        Vec<i32>,
        Vec<i64>,
        Vec<f32>,
        Vec<f64>,
        Vec<bool>,
        Vec<String>
    );

    #[cfg(feature = "uuid")]
    impl_leaf_postgres!(::postgres::Row; uuid::Uuid);
    #[cfg(feature = "uuid")]
    impl_option_leaf_postgres!(::postgres::Row; uuid::Uuid);
    #[cfg(feature = "uuid")]
    impl_leaf_postgres!(::postgres::Row; Vec<uuid::Uuid>);
    #[cfg(feature = "uuid")]
    impl_option_leaf_postgres!(::postgres::Row; Vec<uuid::Uuid>);

    #[cfg(feature = "chrono")]
    impl_leaf_postgres!(::postgres::Row; chrono::NaiveDate, chrono::NaiveTime, chrono::NaiveDateTime, chrono::DateTime<chrono::Utc>);
    #[cfg(feature = "chrono")]
    impl_option_leaf_postgres!(::postgres::Row; chrono::NaiveDate, chrono::NaiveTime, chrono::NaiveDateTime, chrono::DateTime<chrono::Utc>);
    #[cfg(feature = "chrono")]
    impl_leaf_postgres!(::postgres::Row; Vec<chrono::NaiveDate>, Vec<chrono::NaiveTime>, Vec<chrono::NaiveDateTime>, Vec<chrono::DateTime<chrono::Utc>>);
    #[cfg(feature = "chrono")]
    impl_option_leaf_postgres!(::postgres::Row; Vec<chrono::NaiveDate>, Vec<chrono::NaiveTime>, Vec<chrono::NaiveDateTime>, Vec<chrono::DateTime<chrono::Utc>>);

    #[cfg(feature = "serde")]
    impl_leaf_postgres!(::postgres::Row; serde_json::Value);
    #[cfg(feature = "serde")]
    impl_option_leaf_postgres!(::postgres::Row; serde_json::Value);
    #[cfg(feature = "serde")]
    impl_leaf_postgres!(::postgres::Row; Vec<serde_json::Value>);
    #[cfg(feature = "serde")]
    impl_option_leaf_postgres!(::postgres::Row; Vec<serde_json::Value>);

    #[cfg(feature = "rust-decimal")]
    impl_leaf_postgres!(::postgres::Row; rust_decimal::Decimal);
    #[cfg(feature = "rust-decimal")]
    impl_option_leaf_postgres!(::postgres::Row; rust_decimal::Decimal);

    #[cfg(feature = "cidr")]
    impl_leaf_postgres!(::postgres::Row; cidr::IpInet, cidr::IpCidr);
    #[cfg(feature = "cidr")]
    impl_option_leaf_postgres!(::postgres::Row; cidr::IpInet, cidr::IpCidr);

    #[cfg(feature = "geo-types")]
    impl_leaf_postgres!(::postgres::Row; geo_types::Point<f64>, geo_types::LineString<f64>, geo_types::Rect<f64>);
    #[cfg(feature = "geo-types")]
    impl_option_leaf_postgres!(::postgres::Row; geo_types::Point<f64>, geo_types::LineString<f64>, geo_types::Rect<f64>);

    #[cfg(feature = "bit-vec")]
    impl_leaf_postgres!(::postgres::Row; bit_vec::BitVec);
    #[cfg(feature = "bit-vec")]
    impl_option_leaf_postgres!(::postgres::Row; bit_vec::BitVec);

    // Composite (multi-column) Option<T> via NullProbeRow
    impl<T: NullProbeRow<::postgres::Row>> FromDrizzleRow<::postgres::Row> for Option<T> {
        const COLUMN_COUNT: usize = T::COLUMN_COUNT;

        fn from_row_at(row: &::postgres::Row, offset: usize) -> Result<Self, DrizzleError> {
            if T::is_null_at(row, offset)? {
                return Ok(None);
            }
            T::from_row_at(row, offset).map(Some)
        }
    }
}
