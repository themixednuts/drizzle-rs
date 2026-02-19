//! SQLite implementation for Drizzle
//!
//! This crate provides SQLite-specific types, query builders, and utilities.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[allow(unused_imports)]
pub(crate) mod prelude {
    #[cfg(feature = "std")]
    pub use std::{
        borrow::Cow,
        boxed::Box,
        format,
        rc::Rc,
        string::{String, ToString},
        sync::Arc,
        vec,
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
pub mod types {
    pub use drizzle_core::types::*;

    pub type Integer = BigInt;
    pub type Real = Double;
    pub type Blob = Bytes;
}
pub mod values;

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
/// use drizzle_sqlite::params;
///
/// let params = params![{ name: "alice" }, true];
/// ```
#[macro_export]
macro_rules! params {
    // Multiple parameters - creates a fixed-size array of ParamBind values
    [$($param:tt),+ $(,)?] => {
        [
            $(
                $crate::params_internal!($param)
            ),+
        ]
    };
}

/// Internal helper macro for params! - converts individual items to ParamBind values
#[macro_export]
macro_rules! params_internal {
    // Named parameter
    ({ $key:ident: $value:expr }) => {
        $crate::ParamBind::new(stringify!($key), $crate::values::SQLiteValue::from($value))
    };
    // Positional parameter
    ($value:expr) => {
        $crate::ParamBind::positional($crate::values::SQLiteValue::from($value))
    };
}
