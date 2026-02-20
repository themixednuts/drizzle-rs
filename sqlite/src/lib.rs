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
    pub use drizzle_types::sqlite::types::*;
}
pub mod values;

pub use drizzle_core::ParamBind;
