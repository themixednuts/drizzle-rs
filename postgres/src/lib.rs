//! `PostgreSQL` support for drizzle-rs: values, query builders,
//! `PostgreSQL`-only operators and the traits the `#[PostgresTable]` family of
//! macros implement.
//!
//! Most code uses this crate through the `drizzle` crate, which re-exports it
//! as `drizzle::postgres` and adds the database drivers. On its own, this
//! crate builds SQL without running it.

#![allow(unexpected_cfgs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub(crate) mod prelude {
    #[cfg(feature = "std")]
    pub use std::{
        borrow::Cow,
        boxed::Box,
        format,
        rc::Rc,
        string::{String, ToString},
        sync::Arc,
        vec::Vec,
    };

    #[cfg(not(feature = "std"))]
    pub use alloc::{
        borrow::Cow,
        boxed::Box,
        format,
        rc::Rc,
        string::{String, ToString},
        sync::Arc,
        vec::Vec,
    };
}

pub mod attrs;
#[cfg(feature = "aws-data-api")]
pub mod aws_data_api;
pub mod builder;
pub mod common;
pub mod expr;
pub mod helpers;
pub mod traits;
pub mod transaction;
/// SQL type markers for `PostgreSQL` (`Int4`, `Text`, `Jsonb`, ...), from
/// `drizzle_types`.
pub mod types {
    pub use drizzle_types::postgres::types::*;
}
pub mod values;

#[cfg(all(test, feature = "query"))]
mod relational_sql_tests;

#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
pub use postgres::Row;
#[cfg(feature = "tokio-postgres")]
pub use tokio_postgres::Row;

#[doc(hidden)]
pub mod driver_types {
    #[cfg(all(
        any(feature = "serde", feature = "query"),
        feature = "postgres-sync",
        not(feature = "tokio-postgres")
    ))]
    pub use postgres::types::Json;
    #[cfg(all(any(feature = "serde", feature = "query"), feature = "tokio-postgres"))]
    pub use tokio_postgres::types::Json;

    // Wire-codec items for macro-generated `FromSql`/`ToSql` impls, so the
    // expansion never names a driver crate the user may not depend on directly.
    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    pub use bytes::BytesMut;
    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    pub use postgres::types::{FromSql, IsNull, ToSql, Type, to_sql_checked};
    #[cfg(feature = "tokio-postgres")]
    pub use tokio_postgres::types::{FromSql, IsNull, ToSql, Type, to_sql_checked};

    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    pub use field_decode::{FieldDecode, ViaColumn, ViaFromSql};

    /// Decodes a `#[derive(PostgresFromRow)]` field whose type the derive
    /// cannot inspect. Generated code calls
    /// `(&&FieldDecode::<T>::new()).decode_field(row, idx)`: method resolution
    /// tries `ViaColumn` (custom `DrizzlePostgresColumn` types and
    /// `#[PostgresEnum]`s, decoded through their own codec) before
    /// `ViaFromSql` (anything the driver decodes). Both return a decode error
    /// instead of panicking.
    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    mod field_decode {
        use crate::traits::DrizzlePostgresColumn;
        use core::marker::PhantomData;
        use drizzle_core::error::DrizzleError;

        pub struct FieldDecode<T>(PhantomData<T>);

        impl<T> FieldDecode<T> {
            #[must_use]
            pub const fn new() -> Self {
                Self(PhantomData)
            }
        }

        impl<T> Default for FieldDecode<T> {
            fn default() -> Self {
                Self::new()
            }
        }

        pub trait ViaColumn<T> {
            /// Decodes the cell at `idx` through `T`'s column codec.
            ///
            /// # Errors
            ///
            /// Returns the codec's error when the cell does not decode.
            fn decode_field(&self, row: &crate::Row, idx: usize) -> Result<T, DrizzleError>;
        }

        impl<T: DrizzlePostgresColumn> ViaColumn<T> for &FieldDecode<T> {
            fn decode_field(&self, row: &crate::Row, idx: usize) -> Result<T, DrizzleError> {
                T::decode(row, idx)
            }
        }

        pub trait ViaFromSql<T> {
            /// Decodes the cell at `idx` through the driver's `FromSql`.
            ///
            /// # Errors
            ///
            /// Returns [`DrizzleError::ConversionError`] when the cell's wire
            /// type does not match `T` or its value does not decode.
            fn decode_field(&self, row: &crate::Row, idx: usize) -> Result<T, DrizzleError>;
        }

        impl<T: for<'a> super::FromSql<'a>> ViaFromSql<T> for FieldDecode<T> {
            fn decode_field(&self, row: &crate::Row, idx: usize) -> Result<T, DrizzleError> {
                row.try_get::<_, T>(idx)
                    .map_err(|e| DrizzleError::ConversionError(e.to_string().into()))
            }
        }
    }
}

pub use drizzle_core::ParamBind;
pub use transaction::{AccessMode, IsolationLevel, SnapshotId, TransactionConfig};
