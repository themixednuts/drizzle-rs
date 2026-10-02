//! `SQLite` dialect for drizzle-rs.
//!
//! This crate holds the `SQLite` parts of drizzle-rs: the typed query
//! builder ([`builder::QueryBuilder`]), values ([`values::SQLiteValue`]),
//! JSON helpers ([`expr`]), PRAGMA statements ([`pragma`]), and the traits
//! that `#[SQLiteTable]` and friends implement. Most applications use it
//! through the `drizzle` crate (`drizzle::sqlite`), which adds the drivers
//! (`rusqlite`, `libsql`, `turso`).

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[allow(unused_imports)]
pub(crate) mod prelude {
    #[cfg(feature = "std")]
    pub use std::{
        borrow::{Cow, ToOwned},
        boxed::Box,
        format,
        rc::Rc,
        string::{String, ToString},
        sync::Arc,
        vec,
        vec::Vec,
    };

    #[cfg(not(feature = "std"))]
    pub use alloc::{
        borrow::{Cow, ToOwned},
        boxed::Box,
        format,
        rc::Rc,
        string::{String, ToString},
        sync::Arc,
        vec,
        vec::Vec,
    };
}

pub mod attrs;
pub mod builder;
pub mod common;
pub mod connection;
pub mod expr;
pub mod helpers;
pub mod pragma;
pub mod traits;
/// SQL type markers for `SQLite` columns (`Integer`, `Text`, `Blob`, `Real`,
/// `Numeric`, `Any`), re-exported from `drizzle-types`.
pub mod types {
    pub use drizzle_types::sqlite::types::*;
}
pub mod values;

pub use connection::{SQLiteTransactionType, TransactionConfig};
pub use drizzle_core::ParamBind;
