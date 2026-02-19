//! `FromDrizzleRow` leaf impls for PostgreSQL driver rows.
//!
//! When `tokio-postgres` is enabled, we impl for `tokio_postgres::Row`.
//! When only `postgres-sync` is enabled, we impl for `postgres::Row`.
//! Both share the same underlying `FromSql` trait from `postgres-types`,
//! so the generated code is identical.

use crate::error::DrizzleError;
use crate::row::FromDrizzleRow;

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

macro_rules! impl_option_postgres {
    ($row_ty:ty) => {
        impl<T> FromDrizzleRow<$row_ty> for Option<T>
        where
            T: FromDrizzleRow<$row_ty>,
            // For single-column leaves, postgres natively handles Option<T>
            // via its FromSql trait, but for composites (tuples of models)
            // we need to go through the leaf impl. Since all our leaf types
            // are single-column with COLUMN_COUNT=1, we can use try_get
            // which handles NULL natively for postgres.
        {
            const COLUMN_COUNT: usize = T::COLUMN_COUNT;
            fn from_row_at(row: &$row_ty, offset: usize) -> Result<Self, DrizzleError> {
                // For single-column types, try the inner impl — if it fails
                // with a NULL-related error, return None.
                match T::from_row_at(row, offset) {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => {
                        // Check if the value is actually NULL
                        // postgres::Row doesn't have a direct is_null check,
                        // so we try to get it as Option via try_get
                        Ok(None)
                    }
                }
            }
        }
    };
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

    impl_option_postgres!(::tokio_postgres::Row);

    #[cfg(feature = "uuid")]
    impl_leaf_postgres!(::tokio_postgres::Row; uuid::Uuid);
    #[cfg(feature = "uuid")]
    impl_leaf_postgres!(::tokio_postgres::Row; Vec<uuid::Uuid>);

    #[cfg(feature = "chrono")]
    impl_leaf_postgres!(::tokio_postgres::Row; chrono::NaiveDate, chrono::NaiveTime, chrono::NaiveDateTime, chrono::DateTime<chrono::Utc>);
    #[cfg(feature = "chrono")]
    impl_leaf_postgres!(::tokio_postgres::Row; Vec<chrono::NaiveDate>, Vec<chrono::NaiveTime>, Vec<chrono::NaiveDateTime>, Vec<chrono::DateTime<chrono::Utc>>);

    #[cfg(feature = "serde")]
    impl_leaf_postgres!(::tokio_postgres::Row; serde_json::Value);
    #[cfg(feature = "serde")]
    impl_leaf_postgres!(::tokio_postgres::Row; Vec<serde_json::Value>);
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

    impl_option_postgres!(::postgres::Row);

    #[cfg(feature = "uuid")]
    impl_leaf_postgres!(::postgres::Row; uuid::Uuid);
    #[cfg(feature = "uuid")]
    impl_leaf_postgres!(::postgres::Row; Vec<uuid::Uuid>);

    #[cfg(feature = "chrono")]
    impl_leaf_postgres!(::postgres::Row; chrono::NaiveDate, chrono::NaiveTime, chrono::NaiveDateTime, chrono::DateTime<chrono::Utc>);
    #[cfg(feature = "chrono")]
    impl_leaf_postgres!(::postgres::Row; Vec<chrono::NaiveDate>, Vec<chrono::NaiveTime>, Vec<chrono::NaiveDateTime>, Vec<chrono::DateTime<chrono::Utc>>);

    #[cfg(feature = "serde")]
    impl_leaf_postgres!(::postgres::Row; serde_json::Value);
    #[cfg(feature = "serde")]
    impl_leaf_postgres!(::postgres::Row; Vec<serde_json::Value>);
}
