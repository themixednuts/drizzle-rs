//! PostgreSQL support for drizzle-rs
//!
//! This crate provides PostgreSQL-specific types, query builders, and utilities.

#![allow(unexpected_cfgs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
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

    #[cfg(all(feature = "alloc", not(feature = "std")))]
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
pub mod builder;
pub mod common;
pub mod expr;
pub mod helpers;
pub mod traits;
pub mod types {
    pub use drizzle_core::types::*;

    pub type Int2 = SmallInt;
    pub type Int4 = Int;
    pub type Int8 = BigInt;
    pub type Float4 = Float;
    pub type Float8 = Double;
    pub type Varchar = VarChar;
    pub type Bytea = Bytes;
    pub type Boolean = Bool;
    pub type Timestamptz = TimestampTz;
}
pub mod values;

#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
pub use postgres::Row;
#[cfg(feature = "tokio-postgres")]
pub use tokio_postgres::Row;

pub use drizzle_core::ParamBind;

/// Creates an array of SQL parameters for binding values to placeholders.
///
/// # Syntax
/// - `{ name: value }` - Named parameter (creates :name placeholder)
/// - `value` - Positional parameter (creates next positional placeholder)
///
/// # Examples
///
/// ```
/// use drizzle_postgres::params;
///
/// let params = params![{ name: "alice" }, true];
/// ```
#[macro_export]
macro_rules! params {
    [$($param:tt),+ $(,)?] => {
        [
            $(
                $crate::params_internal!($param)
            ),+
        ]
    };
}

/// Internal helper macro for params! - converts items to ParamBind values
#[macro_export]
macro_rules! params_internal {
    ({ $key:ident: $value:expr }) => {
        $crate::ParamBind::new(
            stringify!($key),
            $crate::values::PostgresValue::from($value),
        )
    };
    ($value:expr) => {
        $crate::ParamBind::positional($crate::values::PostgresValue::from($value))
    };
}
