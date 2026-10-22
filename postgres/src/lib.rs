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

    /// Decodes any column type without reading its bytes, so
    /// `Option<AnyNullProbe>` is `None` exactly when the cell is NULL.
    ///
    /// Macro-generated NULL checks use it instead of guessing a concrete
    /// type: a guess that does not match the column's wire type fails
    /// `FromSql::accepts` instead of reporting NULL.
    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    #[derive(Debug, Clone, Copy)]
    pub struct AnyNullProbe;

    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    impl<'a> FromSql<'a> for AnyNullProbe {
        fn from_sql(
            _: &Type,
            _: &'a [u8],
        ) -> Result<Self, Box<dyn core::error::Error + Sync + Send>> {
            Ok(Self)
        }

        fn accepts(_: &Type) -> bool {
            true
        }
    }
}

pub use drizzle_core::ParamBind;
pub use transaction::{AccessMode, IsolationLevel, TransactionConfig};
