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
    pub use drizzle_types::postgres::types::*;
}
pub mod values;

#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
pub use postgres::Row;
#[cfg(feature = "tokio-postgres")]
pub use tokio_postgres::Row;

pub use drizzle_core::ParamBind;
