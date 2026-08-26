//! `MySQL` support for drizzle-rs.
//!
//! This crate is the MySQL dialect boundary. It currently exposes SQL types
//! and client-neutral values; query builders and feature-gated wire adapters
//! layer on those contracts.
//!
//! Wire adapters must set each connection's MySQL session time zone to UTC
//! before executing typed queries. This is the adapter-owned invariant that
//! makes `TIMESTAMP` values round-trip as UTC instants.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub(crate) mod prelude {
    #[cfg(feature = "std")]
    pub use std::{borrow::Cow, boxed::Box, rc::Rc, string::String, sync::Arc, vec::Vec};

    #[cfg(not(feature = "std"))]
    pub use alloc::{borrow::Cow, boxed::Box, rc::Rc, string::String, sync::Arc, vec::Vec};

    #[cfg(all(
        not(feature = "std"),
        any(
            feature = "chrono",
            feature = "time",
            feature = "serde",
            feature = "rust-decimal"
        )
    ))]
    pub use alloc::string::ToString;
}

pub mod attrs;
pub mod common;
pub mod traits;
pub mod types {
    pub use drizzle_types::mysql::types::*;
}

pub mod values;

pub use drizzle_core::{MySQLDialect, ParamBind};
